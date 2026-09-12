//! Tests d'intégration du backend PostgreSQL.
//!
//! Contrairement aux tests unitaires du reste du workspace, ceux-ci
//! exigent une vraie base PostgreSQL accessible via la variable
//! d'environnement `DATABASE_URL` — ils sont donc ignorés par défaut
//! (`#[ignore]`) pour que `cargo test --workspace` continue de
//! fonctionner sans base de données (cf. `docs/DEVELOPMENT_WORKFLOW.md`).
//!
//! Pour les lancer :
//! ```bash
//! export DATABASE_URL=postgres://postgres:agoravote@localhost/agoravote
//! cargo test -p agoravote-store -- --ignored --test-threads=1
//! ```
//! `--test-threads=1` : chaque test insère des données avec des UUID
//! aléatoires (pas de collision entre tests), mais partage la même
//! base — le mode mono-thread évite simplement toute question de
//! contention sur le pool de connexions pendant les tests, pas une
//! histoire d'isolation des données.

use std::collections::HashMap;

use agoravote_core::{Ballot, Campaign, Form, Id, ResultSet};
use agoravote_store::Store;

async fn test_store() -> Store {
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL doit être définie pour lancer ces tests (voir doc du fichier)");
    Store::connect_postgres(&url)
        .await
        .expect("connexion et migration PostgreSQL")
}

#[tokio::test]
#[ignore]
async fn campagne_survit_a_un_aller_retour() {
    let store = test_store().await;
    let campaign = Campaign::new_draft(Id::new_v4(), "Test intégration PostgreSQL");
    let id = campaign.id;

    store.insert_campaign(campaign.clone()).await.unwrap();
    let fetched = store
        .get_campaign(id)
        .await
        .unwrap()
        .expect("campagne relue");

    assert_eq!(fetched.id, campaign.id);
    assert_eq!(fetched.title, campaign.title);
}

#[tokio::test]
#[ignore]
async fn campagne_introuvable_renvoie_none_pas_une_erreur() {
    let store = test_store().await;
    let result = store.get_campaign(Id::new_v4()).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
#[ignore]
async fn update_campaign_persiste_la_mutation() {
    let store = test_store().await;
    let mut campaign = Campaign::new_draft(Id::new_v4(), "À publier");
    let form = Form::new(campaign.id);
    campaign.form_id = Some(form.id);
    let id = campaign.id;
    store.insert_campaign(campaign).await.unwrap();
    store.insert_form(form).await.unwrap();

    let updated = store
        .update_campaign(id, |c| {
            c.publish()
                .expect("publication possible : formulaire présent");
        })
        .await
        .unwrap()
        .expect("la campagne existe");

    assert!(updated.accepts_ballots());

    // Relecture depuis une nouvelle requête : vérifie que la mutation
    // a bien été écrite en base, pas seulement retournée en mémoire
    // par la transaction qui vient de s'exécuter.
    let reloaded = store.get_campaign(id).await.unwrap().unwrap();
    assert!(reloaded.accepts_ballots());
}

#[tokio::test]
#[ignore]
async fn update_campaign_sur_id_inconnu_renvoie_none() {
    let store = test_store().await;
    let result = store.update_campaign(Id::new_v4(), |_| {}).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
#[ignore]
async fn ballots_sont_filtres_par_campagne_et_question() {
    let store = test_store().await;
    let campaign_id = Id::new_v4();
    let question_a = Id::new_v4();
    let question_b = Id::new_v4();

    store
        .add_ballot(Ballot::for_selections(
            campaign_id,
            question_a,
            None,
            vec!["oui".into()],
        ))
        .await
        .unwrap();
    store
        .add_ballot(Ballot::for_selections(
            campaign_id,
            question_a,
            None,
            vec!["non".into()],
        ))
        .await
        .unwrap();
    store
        .add_ballot(Ballot::for_selections(
            campaign_id,
            question_b,
            None,
            vec!["autre".into()],
        ))
        .await
        .unwrap();

    let ballots_a = store
        .ballots_for_question(campaign_id, question_a)
        .await
        .unwrap();
    let ballots_b = store
        .ballots_for_question(campaign_id, question_b)
        .await
        .unwrap();

    assert_eq!(ballots_a.len(), 2);
    assert_eq!(ballots_b.len(), 1);
}

#[tokio::test]
#[ignore]
async fn store_result_ecrase_le_resultat_precedent_de_la_meme_question() {
    let store = test_store().await;
    let campaign_id = Id::new_v4();
    let question_id = Id::new_v4();

    let make_result = |winner: &str| {
        let mut percentages = HashMap::new();
        percentages.insert(winner.to_string(), 100.0);
        ResultSet::new(
            campaign_id,
            question_id,
            "voting.majority",
            "1.0.0",
            agoravote_core::voting_method::TallyOutcome {
                total_ballots: 1,
                valid_ballots: 1,
                winners: vec![winner.to_string()],
                percentages,
                counts: HashMap::new(),
                quorum_met: None,
                metadata: HashMap::new(),
            },
        )
    };

    store
        .store_result(make_result("premier_calcul"))
        .await
        .unwrap();
    store.store_result(make_result("recalcul")).await.unwrap();

    let result = store.get_result(question_id).await.unwrap().unwrap();
    assert_eq!(result.outcome.winners, vec!["recalcul".to_string()]);
}

#[tokio::test]
#[ignore]
async fn compte_survit_a_un_aller_retour_et_email_est_unique() {
    let store = test_store().await;
    let user = agoravote_core::User::new(Id::new_v4(), "Test Auth");
    let user_id = user.id;
    store.insert_user(user).await.unwrap();

    let email = format!("test-{}@example.invalid", Id::new_v4());
    let account = agoravote_core::Account::new(user_id, &email, "empreinte-fictive");
    store.insert_account(account.clone()).await.unwrap();

    let fetched = store
        .get_account_by_email(&email)
        .await
        .unwrap()
        .expect("compte relu");
    assert_eq!(fetched.user_id, user_id);
    assert_eq!(fetched.password_hash, "empreinte-fictive");

    // Un second compte avec le même email doit être refusé (contrainte
    // UNIQUE, cf. migration 0002_auth.sql) — pas silencieusement
    // écrasé.
    let duplicate = agoravote_core::Account::new(Id::new_v4(), &email, "autre-empreinte");
    let result = store.insert_account(duplicate).await;
    assert!(matches!(
        result,
        Err(agoravote_store::StoreError::EmailAlreadyExists)
    ));
}

#[tokio::test]
#[ignore]
async fn session_peut_etre_creee_lue_puis_supprimee() {
    let store = test_store().await;
    let user = agoravote_core::User::new(Id::new_v4(), "Test Session");
    let user_id = user.id;
    let organization_id = user.organization_id;
    store.insert_user(user).await.unwrap();

    let (_, session) = agoravote_auth::issue_session(user_id, organization_id);
    let token_hash = session.token_hash.clone();
    store.insert_session(session).await.unwrap();

    let fetched = store
        .get_session(&token_hash)
        .await
        .unwrap()
        .expect("session relue");
    assert_eq!(fetched.user_id, user_id);

    store.delete_session(&token_hash).await.unwrap();
    assert!(store.get_session(&token_hash).await.unwrap().is_none());
}

#[tokio::test]
#[ignore]
async fn lien_g1_survit_a_un_aller_retour_et_la_cle_publique_est_unique() {
    let store = test_store().await;
    let user = agoravote_core::User::new(Id::new_v4(), "Test Ğ1");
    let user_id = user.id;
    store.insert_user(user).await.unwrap();

    let public_key_hex = format!("{}{}", Id::new_v4().simple(), Id::new_v4().simple());
    let link = agoravote_core::G1Link::new(user_id, &public_key_hex);
    store.insert_g1_link(link).await.unwrap();

    let fetched = store
        .get_g1_link_by_public_key(&public_key_hex)
        .await
        .unwrap()
        .expect("lien relu");
    assert_eq!(fetched.user_id, user_id);

    // Un second lien vers la même clé publique doit être refusé (elle
    // ne peut être liée qu'à un seul utilisateur AgoraVote) — pas
    // silencieusement écrasé, même logique que pour l'email unique
    // d'un compte classique.
    let other_user = agoravote_core::User::new(Id::new_v4(), "Autre utilisateur");
    let other_user_id = other_user.id;
    store.insert_user(other_user).await.unwrap();
    let duplicate = agoravote_core::G1Link::new(other_user_id, &public_key_hex);
    let result = store.insert_g1_link(duplicate).await;
    assert!(matches!(
        result,
        Err(agoravote_store::StoreError::G1PublicKeyAlreadyLinked)
    ));
}

#[tokio::test]
#[ignore]
async fn lien_g1_absent_renvoie_none_pas_une_erreur() {
    let store = test_store().await;
    let result = store
        .get_g1_link_by_public_key("clé-inexistante")
        .await
        .unwrap();
    assert!(result.is_none());
}
