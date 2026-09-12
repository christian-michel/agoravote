//! Backend en mémoire — cf. doc du module parent ([`crate`]).
//!
//! Contenu déplacé tel quel depuis l'ancien
//! `agoravote-api/src/state.rs` (voir `docs/DEVLOG.md`, entrée
//! "persistance PostgreSQL" pour l'historique de ce déplacement). Le
//! comportement observable ne change pas : toujours un
//! `Mutex<HashMap<...>>`, toujours la même politique de récupération
//! sur mutex empoisonné.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use agoravote_core::{Account, Ballot, Campaign, Form, G1Link, Id, ResultSet, Session, User};

use crate::StoreError;

#[derive(Clone)]
pub struct MemoryStore {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    campaigns: HashMap<Id, Campaign>,
    forms: HashMap<Id, Form>,
    ballots_by_campaign: HashMap<Id, Vec<Ballot>>,
    results_by_question: HashMap<Id, ResultSet>,
    users: HashMap<Id, User>,
    accounts_by_email: HashMap<String, Account>,
    sessions_by_token_hash: HashMap<String, Session>,
    g1_links_by_public_key: HashMap<String, G1Link>,
}

/// Verrouille le `Mutex` en récupérant les données même s'il a été
/// empoisonné par une panique précédente — cf. l'explication complète
/// dans `docs/LOGGING.md` (section "erreurs silencieuses") : un mutex
/// empoisonné ne doit jamais transformer un incident isolé en panne
/// durable de toute l'API.
fn lock_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
        }
    }

    // Les méthodes ci-dessous sont déclarées `async fn` pour respecter
    // l'interface commune avec `PgStore` (cf. `Store` dans `lib.rs`),
    // mais ne contiennent aucun véritable point d'attente : verrouiller
    // un `Mutex` en mémoire est instantané. Le mot-clé `async` a donc
    // ici un coût nul à l'exécution (pas d'I/O réelle), seulement un
    // habillage nécessaire à l'uniformité de l'interface.

    pub async fn insert_campaign(&self, campaign: Campaign) -> Result<(), StoreError> {
        lock_recover(&self.inner)
            .campaigns
            .insert(campaign.id, campaign);
        Ok(())
    }

    pub async fn get_campaign(&self, id: Id) -> Result<Option<Campaign>, StoreError> {
        Ok(lock_recover(&self.inner).campaigns.get(&id).cloned())
    }

    pub async fn update_campaign<F>(&self, id: Id, f: F) -> Result<Option<Campaign>, StoreError>
    where
        F: FnOnce(&mut Campaign) + Send,
    {
        let mut inner = lock_recover(&self.inner);
        let Some(campaign) = inner.campaigns.get_mut(&id) else {
            return Ok(None);
        };
        f(campaign);
        Ok(Some(campaign.clone()))
    }

    pub async fn insert_form(&self, form: Form) -> Result<(), StoreError> {
        lock_recover(&self.inner).forms.insert(form.id, form);
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_form(&self, id: Id) -> Result<Option<Form>, StoreError> {
        Ok(lock_recover(&self.inner).forms.get(&id).cloned())
    }

    pub async fn form_for_campaign(&self, campaign_id: Id) -> Result<Option<Form>, StoreError> {
        Ok(lock_recover(&self.inner)
            .forms
            .values()
            .find(|f| f.campaign_id == campaign_id)
            .cloned())
    }

    pub async fn add_ballot(&self, ballot: Ballot) -> Result<(), StoreError> {
        lock_recover(&self.inner)
            .ballots_by_campaign
            .entry(ballot.campaign_id)
            .or_default()
            .push(ballot);
        Ok(())
    }

    pub async fn ballots_for_question(
        &self,
        campaign_id: Id,
        question_id: Id,
    ) -> Result<Vec<Ballot>, StoreError> {
        Ok(lock_recover(&self.inner)
            .ballots_by_campaign
            .get(&campaign_id)
            .map(|ballots| {
                ballots
                    .iter()
                    .filter(|b| b.question_id == question_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn store_result(&self, result: ResultSet) -> Result<(), StoreError> {
        lock_recover(&self.inner)
            .results_by_question
            .insert(result.question_id, result);
        Ok(())
    }

    pub async fn get_result(&self, question_id: Id) -> Result<Option<ResultSet>, StoreError> {
        Ok(lock_recover(&self.inner)
            .results_by_question
            .get(&question_id)
            .cloned())
    }

    // --- Authentification (§13) ---

    pub async fn insert_user(&self, user: User) -> Result<(), StoreError> {
        lock_recover(&self.inner).users.insert(user.id, user);
        Ok(())
    }

    pub async fn get_user(&self, id: Id) -> Result<Option<User>, StoreError> {
        Ok(lock_recover(&self.inner).users.get(&id).cloned())
    }

    pub async fn insert_account(&self, account: Account) -> Result<(), StoreError> {
        let mut inner = lock_recover(&self.inner);
        // Reproduit la contrainte `UNIQUE` du backend PostgreSQL (cf.
        // migration `0002_auth.sql`) pour que les deux backends aient
        // exactement le même comportement observable — sans cette
        // vérification, ce backend accepterait silencieusement deux
        // comptes avec le même email (dernier arrivé écrase le
        // premier dans la `HashMap`), ce que PostgreSQL refuse.
        if inner.accounts_by_email.contains_key(&account.email) {
            return Err(StoreError::EmailAlreadyExists);
        }
        inner
            .accounts_by_email
            .insert(account.email.clone(), account);
        Ok(())
    }

    pub async fn get_account_by_email(&self, email: &str) -> Result<Option<Account>, StoreError> {
        Ok(lock_recover(&self.inner)
            .accounts_by_email
            .get(email)
            .cloned())
    }

    pub async fn insert_session(&self, session: Session) -> Result<(), StoreError> {
        lock_recover(&self.inner)
            .sessions_by_token_hash
            .insert(session.token_hash.clone(), session);
        Ok(())
    }

    pub async fn get_session(&self, token_hash: &str) -> Result<Option<Session>, StoreError> {
        Ok(lock_recover(&self.inner)
            .sessions_by_token_hash
            .get(token_hash)
            .cloned())
    }

    pub async fn delete_session(&self, token_hash: &str) -> Result<(), StoreError> {
        lock_recover(&self.inner)
            .sessions_by_token_hash
            .remove(token_hash);
        Ok(())
    }

    // --- Identité Ğ1v2 optionnelle (addendum v0.3) ---

    pub async fn insert_g1_link(&self, link: G1Link) -> Result<(), StoreError> {
        let mut inner = lock_recover(&self.inner);
        // Même raisonnement que `insert_account` ci-dessus : reproduit
        // la contrainte `UNIQUE` du backend PostgreSQL (cf. migration
        // `0003_g1_link.sql`) pour un comportement identique entre les
        // deux backends.
        if inner
            .g1_links_by_public_key
            .contains_key(&link.public_key_hex)
        {
            return Err(StoreError::G1PublicKeyAlreadyLinked);
        }
        inner
            .g1_links_by_public_key
            .insert(link.public_key_hex.clone(), link);
        Ok(())
    }

    pub async fn get_g1_link_by_public_key(
        &self,
        public_key_hex: &str,
    ) -> Result<Option<G1Link>, StoreError> {
        Ok(lock_recover(&self.inner)
            .g1_links_by_public_key
            .get(public_key_hex)
            .cloned())
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
