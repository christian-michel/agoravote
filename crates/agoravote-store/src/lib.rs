//! # agoravote-store
//!
//! Persistance d'AgoraVote — cf. cahier des charges §12.2
//! (« Déploiement » : PostgreSQL en mode serveur, base embarquée
//! possible pour certains modes locaux).
//!
//! Ce crate expose un seul type public, [`Store`], qui se comporte de
//! façon identique quel que soit le backend réellement utilisé
//! derrière :
//!
//! - [`memory::MemoryStore`] — tout en RAM, perdu à l'arrêt du
//!   processus. C'est l'ancien contenu de
//!   `agoravote-api/src/state.rs`, déplacé ici tel quel : le
//!   comportement observable de l'API n'a pas changé quand ce backend
//!   est utilisé, seul son emplacement dans le code a bougé.
//! - [`postgres::PgStore`] — persistant, backend cible pour tout usage
//!   au-delà d'une démonstration locale (cf. §12.2).
//!
//! ## Pourquoi un `enum` plutôt qu'un trait `dyn Store` ?
//!
//! Rust 1.75 (cf. note de toolchain dans `Cargo.toml` racine) ne
//! permet pas nativement les fonctions `async` dans un trait objet
//! (`dyn Trait`) sans dépendance supplémentaire (`async-trait`, avec
//! son coût d'allocation par appel). Avec seulement deux backends,
//! prévus pour ne pas grandir en nombre à court terme, un `enum` avec
//! un `match` dans chaque méthode est plus simple, plus rapide, et
//! tout aussi facile à étendre qu'un trait — au prix d'un `match`
//! répété dans chaque méthode, jugé acceptable ici.
//!
//! ## Comment `agoravote-api` choisit son backend
//!
//! `main.rs` regarde la variable d'environnement `DATABASE_URL` : si
//! elle est définie, [`Store::connect_postgres`] est utilisé (avec
//! migration automatique au démarrage) ; sinon,
//! [`Store::new_memory`] — préservant le mode "zéro dépendance" déjà
//! documenté (cahier des charges §1.1, "auto-hébergement / exécution
//! locale").

pub mod memory;
pub mod postgres;

use agoravote_core::{Account, Ballot, Campaign, Form, G1Link, Id, ResultSet, Session, User};

/// Erreurs de persistance. Volontairement peu détaillé pour l'appelant
/// HTTP (`agoravote-api`) : le détail exact (quelle requête SQL a
/// échoué) va dans les logs via `tracing::error!` au moment où
/// l'erreur est produite (cf. `postgres.rs`), pas dans le message
/// renvoyé au client — un client de l'API n'a pas à connaître le
/// schéma SQL interne.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("erreur de connexion ou de requête à la base de données")]
    Database(#[from] sqlx::Error),

    #[error("erreur de (dé)sérialisation des données stockées")]
    Serialization(#[from] serde_json::Error),

    #[error("erreur de migration de schéma")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// Distingué d'une erreur générique `Database` pour que
    /// `agoravote-api` puisse renvoyer un message utile ("cet email
    /// est déjà utilisé", 409) plutôt que l'erreur 500 générique et
    /// opaque réservée aux vraies pannes de persistance — cf.
    /// `routes.rs::internal_error` et son commentaire sur le fait de
    /// ne jamais fuiter de détail technique brut au client.
    #[error("un compte existe déjà avec cette adresse e-mail")]
    EmailAlreadyExists,

    /// Même raisonnement que `EmailAlreadyExists` ci-dessus, pour la
    /// contrainte d'unicité de `g1_links.public_key_hex` (cf. migration
    /// `0003_g1_link.sql`) : un attaquant qui rejouerait une preuve
    /// Ğ1 déjà liée à un autre compte doit recevoir un message utile,
    /// pas une 500 générique.
    #[error("cette clé Ğ1 est déjà liée à un autre compte")]
    G1PublicKeyAlreadyLinked,
}

/// Point d'accès unique à la persistance, quel que soit le backend.
#[derive(Clone)]
pub enum Store {
    Memory(memory::MemoryStore),
    Postgres(postgres::PgStore),
}

impl Store {
    /// Backend en mémoire — cf. `docs/ARCHITECTURE.md` pour son statut
    /// (tests, démonstration locale sans dépendance externe).
    pub fn new_memory() -> Self {
        Self::Memory(memory::MemoryStore::new())
    }

    /// Backend PostgreSQL : ouvre un pool de connexions vers
    /// `database_url` et applique les migrations embarquées
    /// (`migrations/`) si elles ne l'ont pas déjà été. Un déploiement
    /// répété (redémarrage du conteneur) est donc sûr : les migrations
    /// déjà appliquées ne sont jamais rejouées.
    pub async fn connect_postgres(database_url: &str) -> Result<Self, StoreError> {
        let store = postgres::PgStore::connect(database_url).await?;
        Ok(Self::Postgres(store))
    }

    pub async fn insert_campaign(&self, campaign: Campaign) -> Result<(), StoreError> {
        match self {
            Self::Memory(s) => s.insert_campaign(campaign).await,
            Self::Postgres(s) => s.insert_campaign(campaign).await,
        }
    }

    pub async fn get_campaign(&self, id: Id) -> Result<Option<Campaign>, StoreError> {
        match self {
            Self::Memory(s) => s.get_campaign(id).await,
            Self::Postgres(s) => s.get_campaign(id).await,
        }
    }

    /// Applique une fonction de mutation à une campagne existante et
    /// renvoie la campagne mise à jour, ou `None` si elle n'existe pas.
    ///
    /// `F: Send` est requis par le backend PostgreSQL, dont
    /// l'implémentation traverse un point d'attente asynchrone
    /// (lecture, puis écriture) entre la lecture et l'appel à `f` —
    /// contrainte que le backend mémoire n'a pas mais respecte aussi
    /// par cohérence d'interface.
    pub async fn update_campaign<F>(&self, id: Id, f: F) -> Result<Option<Campaign>, StoreError>
    where
        F: FnOnce(&mut Campaign) + Send,
    {
        match self {
            Self::Memory(s) => s.update_campaign(id, f).await,
            Self::Postgres(s) => s.update_campaign(id, f).await,
        }
    }

    pub async fn insert_form(&self, form: Form) -> Result<(), StoreError> {
        match self {
            Self::Memory(s) => s.insert_form(form).await,
            Self::Postgres(s) => s.insert_form(form).await,
        }
    }

    /// Non encore branché sur une route HTTP — cf. même note que dans
    /// l'ancien `agoravote-api/src/state.rs`.
    #[allow(dead_code)]
    pub async fn get_form(&self, id: Id) -> Result<Option<Form>, StoreError> {
        match self {
            Self::Memory(s) => s.get_form(id).await,
            Self::Postgres(s) => s.get_form(id).await,
        }
    }

    /// Utilisé par `GET /campaigns/:id/form` (écran "03. Éditeur de
    /// formulaire" en lecture, et écran "09. Vote citoyen" côté
    /// frontend — cf. `docs/openapi.yaml`).
    pub async fn form_for_campaign(&self, campaign_id: Id) -> Result<Option<Form>, StoreError> {
        match self {
            Self::Memory(s) => s.form_for_campaign(campaign_id).await,
            Self::Postgres(s) => s.form_for_campaign(campaign_id).await,
        }
    }

    pub async fn add_ballot(&self, ballot: Ballot) -> Result<(), StoreError> {
        match self {
            Self::Memory(s) => s.add_ballot(ballot).await,
            Self::Postgres(s) => s.add_ballot(ballot).await,
        }
    }

    pub async fn ballots_for_question(
        &self,
        campaign_id: Id,
        question_id: Id,
    ) -> Result<Vec<Ballot>, StoreError> {
        match self {
            Self::Memory(s) => s.ballots_for_question(campaign_id, question_id).await,
            Self::Postgres(s) => s.ballots_for_question(campaign_id, question_id).await,
        }
    }

    pub async fn store_result(&self, result: ResultSet) -> Result<(), StoreError> {
        match self {
            Self::Memory(s) => s.store_result(result).await,
            Self::Postgres(s) => s.store_result(result).await,
        }
    }

    pub async fn get_result(&self, question_id: Id) -> Result<Option<ResultSet>, StoreError> {
        match self {
            Self::Memory(s) => s.get_result(question_id).await,
            Self::Postgres(s) => s.get_result(question_id).await,
        }
    }

    // --- Authentification (§13) ---

    pub async fn insert_user(&self, user: User) -> Result<(), StoreError> {
        match self {
            Self::Memory(s) => s.insert_user(user).await,
            Self::Postgres(s) => s.insert_user(user).await,
        }
    }

    pub async fn get_user(&self, id: Id) -> Result<Option<User>, StoreError> {
        match self {
            Self::Memory(s) => s.get_user(id).await,
            Self::Postgres(s) => s.get_user(id).await,
        }
    }

    /// Crée un compte. Échoue avec `StoreError::Database` si l'email
    /// existe déjà (contrainte `UNIQUE` en base, cf. migration
    /// `0002_auth.sql`) — c'est la seule protection réellement fiable
    /// contre une double inscription concurrente (deux requêtes
    /// d'inscription simultanées avec le même email : un simple
    /// `get_account_by_email` avant insertion, sans contrainte en
    /// base, laisserait une fenêtre de course entre la vérification et
    /// l'écriture).
    pub async fn insert_account(&self, account: Account) -> Result<(), StoreError> {
        match self {
            Self::Memory(s) => s.insert_account(account).await,
            Self::Postgres(s) => s.insert_account(account).await,
        }
    }

    /// Recherche par email — l'email est comparé tel que fourni : la
    /// normalisation (minuscules) est de la responsabilité de
    /// l'appelant (`agoravote-api`), pas de ce store, pour que le
    /// comportement soit identique entre les deux backends sans
    /// dépendre d'une fonction SQL spécifique (`lower(...)`) qui
    /// n'existe pas en mémoire.
    pub async fn get_account_by_email(&self, email: &str) -> Result<Option<Account>, StoreError> {
        match self {
            Self::Memory(s) => s.get_account_by_email(email).await,
            Self::Postgres(s) => s.get_account_by_email(email).await,
        }
    }

    pub async fn insert_session(&self, session: Session) -> Result<(), StoreError> {
        match self {
            Self::Memory(s) => s.insert_session(session).await,
            Self::Postgres(s) => s.insert_session(session).await,
        }
    }

    pub async fn get_session(&self, token_hash: &str) -> Result<Option<Session>, StoreError> {
        match self {
            Self::Memory(s) => s.get_session(token_hash).await,
            Self::Postgres(s) => s.get_session(token_hash).await,
        }
    }

    /// Supprime une session (déconnexion explicite). Idempotent :
    /// supprimer une session déjà absente n'est pas une erreur.
    pub async fn delete_session(&self, token_hash: &str) -> Result<(), StoreError> {
        match self {
            Self::Memory(s) => s.delete_session(token_hash).await,
            Self::Postgres(s) => s.delete_session(token_hash).await,
        }
    }

    // --- Identité Ğ1v2 optionnelle (addendum v0.3) ---

    /// Lie une clé publique Ğ1 à un utilisateur existant. Échoue avec
    /// `StoreError::G1PublicKeyAlreadyLinked` si cette clé est déjà
    /// liée à un compte (le sien ou un autre) — appelé uniquement
    /// après vérification de la preuve de possession de clé
    /// (`agoravote_g1::verify_signature`), jamais sur une simple
    /// déclaration non prouvée.
    pub async fn insert_g1_link(&self, link: G1Link) -> Result<(), StoreError> {
        match self {
            Self::Memory(s) => s.insert_g1_link(link).await,
            Self::Postgres(s) => s.insert_g1_link(link).await,
        }
    }

    pub async fn get_g1_link_by_public_key(
        &self,
        public_key_hex: &str,
    ) -> Result<Option<G1Link>, StoreError> {
        match self {
            Self::Memory(s) => s.get_g1_link_by_public_key(public_key_hex).await,
            Self::Postgres(s) => s.get_g1_link_by_public_key(public_key_hex).await,
        }
    }
}
