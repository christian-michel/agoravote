//! Routes HTTP de l'API de démonstration.
//!
//! Chaque handler est volontairement fin : il ne fait que (1) valider
//! la forme de la requête, (2) appeler le noyau (`agoravote-core`,
//! `agoravote-voting`, `agoravote-stats`) et (3) mettre en forme la
//! réponse JSON. Toute la logique métier réelle (transitions de
//! statut, calcul de résultat...) vit dans les crates de domaine, pas
//! ici — cf. cahier des charges §4, principe de séparation des
//! couches, appliqué ici à la frontière API/domaine.
//!
//! ## Cartographie routes ↔ écrans (§10.1)
//!
//! | Route                                              | Écran(s)                        |
//! |-----------------------------------------------------|----------------------------------|
//! | `POST /auth/register`                                | (nouveau, §13)                    |
//! | `POST /auth/login`                                   | (nouveau, §13)                    |
//! | `POST /auth/logout`                                  | (nouveau, §13)                    |
//! | `GET  /auth/me`                                       | en-tête (rechargement de page)    |
//! | `POST /campaigns`                                    | 01 Tableau de bord / 02 Campagnes |
//! | `POST /campaigns/:id/form`                            | 03 Éditeur de formulaire          |
//! | `POST /campaigns/:id/publish`                          | 02 Campagnes                      |
//! | `POST /campaigns/:id/close`                            | 02 Campagnes                      |
//! | `POST /campaigns/:id/questions/:qid/ballots`             | 09 Vote citoyen                   |
//! | `POST /campaigns/:id/questions/:qid/tally`               | 06 Configuration du scrutin       |
//! | `GET  /campaigns/:id/questions/:qid/results`             | 11 Résultats en direct            |
//! | `GET  /modules`                                        | 16 Modules                        |
//!
//! ## Routes protégées (§13)
//!
//! Créer/publier/clôturer une campagne, créer un formulaire et lancer
//! un dépouillement exigent désormais un jeton valide (en-tête
//! `Authorization: Bearer <jeton>`) porté par un utilisateur ayant le
//! rôle `Organizer` ou `Admin` — cf. `auth.rs`. Consulter une
//! campagne/un résultat et voter restent accessibles sans compte
//! (§3.2 : un sondage public n'exige pas toujours une identité), mais
//! voter avec un jeton valide associe le bulletin à l'utilisateur et
//! empêche un second vote sur la même question (cf. `cast_ballot`).

use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, State};
use axum::http::{HeaderName, Request, StatusCode};
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::Span;

use agoravote_core::ballot::Ballot;
use agoravote_core::campaign::Campaign;
use agoravote_core::form::Form;
use agoravote_core::result::ResultSet;
use agoravote_core::voting_method::VotingParams;
use agoravote_core::{Account, G1Link, Id, User};

use crate::auth::{require_organizer, AuthUser, OptionalAuthUser};
use crate::dto::{
    AuthResponse, CastBallotRequest, CreateCampaignRequest, CreateFormRequest, ErrorResponse,
    G1ChallengeResponse, G1VerifyRequest, LoginRequest, RegisterRequest, TallyRequest,
};
use crate::state::AppState;

/// Nom du header HTTP utilisé pour porter l'identifiant de requête
/// (généré côté serveur si absent). Le renvoyer au client — via
/// `PropagateRequestIdLayer` ci-dessous — permet à un utilisateur qui
/// signale un bug de fournir CET identifiant, qu'on peut alors
/// retrouver instantanément dans les logs serveur, plutôt que de
/// devoir corréler par date/heure approximative.
static REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Construit le routeur complet de l'API. Séparé de `main.rs` pour
/// être testable indépendamment du serveur HTTP réel (cf. tests
/// d'intégration ci-dessous, `mod tests`, qui exercent ce routeur avec
/// `tower::ServiceExt::oneshot` sans jamais ouvrir de vrai port TCP).
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/modules", get(list_modules))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/auth/g1/challenge", post(g1_challenge))
        .route("/auth/g1/verify", post(g1_verify))
        .route("/campaigns", post(create_campaign))
        .route("/campaigns/:campaign_id", get(get_campaign))
        .route(
            "/campaigns/:campaign_id/form",
            post(create_form).get(get_form),
        )
        .route("/campaigns/:campaign_id/publish", post(publish_campaign))
        .route("/campaigns/:campaign_id/close", post(close_campaign))
        .route(
            "/campaigns/:campaign_id/questions/:question_id/ballots",
            post(cast_ballot),
        )
        .route(
            "/campaigns/:campaign_id/questions/:question_id/tally",
            post(tally_question),
        )
        .route(
            "/campaigns/:campaign_id/questions/:question_id/results",
            get(get_results),
        )
        .with_state(state)
        // --- Observabilité (cf. docs/LOGGING.md pour le détail) ---
        //
        // ORDRE DES COUCHES — le point le plus piégeux de tout ce
        // fichier, volontairement documenté en détail car il a été la
        // source d'un vrai bug pendant le développement (voir
        // docs/DEVLOG.md, entrée correspondante) :
        //
        // `Router::layer` empile les couches de la DERNIÈRE appelée
        // (la plus extérieure, elle voit la requête EN PREMIER) à la
        // PREMIÈRE appelée (la plus intérieure, tout contre le
        // routeur). Pour que `PropagateRequestIdLayer` puisse copier
        // l'id de requête vers la réponse, encore faut-il que
        // `SetRequestIdLayer` ait déjà posé cet id AVANT que
        // `PropagateRequestIdLayer` ne voie passer la requête — donc
        // `SetRequestIdLayer` doit être plus EXTÉRIEUR, donc appelé en
        // DERNIER. La première version de ce fichier faisait l'inverse
        // (`SetRequestIdLayer` appelé en premier = intérieur) : les
        // réponses ne portaient jamais d'id, en silence, jusqu'à ce que
        // le test `request_id_est_propage_dans_la_reponse` ci-dessous
        // le révèle. Ordre extérieur → intérieur voulu, tel qu'appliqué
        // ci-dessous du dernier au premier `.layer()` :
        //
        //   1. SetRequestIdLayer   (pose l'id, doit tout voir en premier)
        //   2. TraceLayer          (journalise, veut l'id déjà posé)
        //   3. PropagateRequestIdLayer (recopie l'id sur la réponse,
        //                               y compris une réponse générée
        //                               plus loin par CatchPanic/Timeout)
        //   4. CatchPanicLayer     (rattrape les panics des handlers)
        //   5. DefaultBodyLimit
        //   6. TimeoutLayer        (le plus proche du routeur)
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        // Cf. commentaire détaillé dans `main.rs::init_observability` :
        // resserre la limite de corps de requête par défaut d'axum
        // (2 Mo, cf. docs/SECURITY.md) à une valeur adaptée aux petites
        // charges utiles JSON de cette API.
        .layer(DefaultBodyLimit::max(64 * 1024))
        // Convertit toute panique survenant DANS un handler en réponse
        // HTTP 500 + log `tracing::error!`, plutôt qu'en coupure brutale
        // de la connexion sans aucune réponse ni trace exploitable —
        // c'est le complément, côté "une seule requête", du hook de
        // panique global installé dans `main.rs` (qui, lui, couvre tout
        // le processus). Voir le test
        // `panique_dans_un_handler_devient_une_reponse_500` ci-dessous.
        .layer(CatchPanicLayer::new())
        .layer(PropagateRequestIdLayer::new(REQUEST_ID_HEADER.clone()))
        .layer(
            TraceLayer::new_for_http()
                // Un "span" par requête : toutes les lignes de log
                // émises pendant le traitement héritent automatiquement
                // de ces champs, sans avoir à les répéter à chaque
                // `tracing::info!` dans les handlers.
                .make_span_with(|request: &Request<_>| {
                    let request_id = request
                        .headers()
                        .get(&REQUEST_ID_HEADER)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("inconnu")
                        .to_string();
                    let client_ip = request
                        .extensions()
                        .get::<ConnectInfo<SocketAddr>>()
                        .map(|ci| ci.0.to_string())
                        .unwrap_or_else(|| "inconnu".to_string());
                    tracing::info_span!(
                        "requete_http",
                        id = %request_id,
                        methode = %request.method(),
                        chemin = %request.uri().path(),
                        client_ip = %client_ip,
                    )
                })
                // Émis pour CHAQUE requête terminée, succès ou échec —
                // c'est ce qui rend un 404/400 visible côté serveur
                // (voir aussi les `tracing::warn!` explicites dans
                // `not_found`/`bad_request` plus bas, qui ajoutent le
                // message métier précis à l'intérieur de ce span).
                .on_response(
                    |response: &axum::http::Response<_>, latency: Duration, _span: &Span| {
                        tracing::info!(
                            statut = response.status().as_u16(),
                            latence_ms = latency.as_millis(),
                            "requête traitée"
                        );
                    },
                )
                // Ne se déclenche QUE pour les échecs serveur (5xx par
                // défaut) ou les connexions coupées anormalement — donc
                // idéal pour une alerte de supervision ("erreur" plutôt
                // que "info"), sans notifier sur les 400/404 attendus.
                .on_failure(
                    |error: tower_http::classify::ServerErrorsFailureClass,
                     latency: Duration,
                     _span: &Span| {
                        tracing::error!(
                            erreur = %error,
                            latence_ms = latency.as_millis(),
                            "échec serveur détecté"
                        );
                    },
                ),
        )
        .layer(SetRequestIdLayer::new(
            REQUEST_ID_HEADER.clone(),
            MakeRequestUuid,
        ))
}

async fn health() -> &'static str {
    "ok"
}

/// `GET /modules` — écran "16. Modules" : catalogue des méthodes de
/// vote installées, avec leur manifeste complet (§6).
async fn list_modules(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "voting_methods": state.voting_methods.manifests() }))
}

/// `POST /auth/register` — crée un `User` (profil, rôle `Organizer`
/// par défaut — cf. note ci-dessous) et un `Account` (identifiants),
/// puis renvoie immédiatement une session (pas besoin de se
/// reconnecter juste après s'être inscrit).
///
/// Rôle par défaut `Organizer`, pas `Voter` : ce MVP n'a pas d'écran
/// d'invitation d'organisateurs par un administrateur (§21, travaux
/// restants) — quiconque s'inscrit aujourd'hui le fait dans l'intention
/// de créer des campagnes. À revoir dès qu'un vrai flux d'invitation
/// existera.
async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    if req.password.len() < 8 {
        return Err(bad_request(
            "le mot de passe doit contenir au moins 8 caractères",
        ));
    }

    // Normalisation : deux inscriptions avec "Alice@Example.com" et
    // "[email protected]" doivent être refusées comme un doublon — cf.
    // le commentaire de `Store::get_account_by_email` sur le partage
    // de cette responsabilité entre l'API et le store.
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(bad_request("adresse e-mail invalide"));
    }

    let password_hash = agoravote_auth::hash_password(&req.password).map_err(|e| {
        tracing::error!(erreur = %e, "échec du hachage de mot de passe");
        internal_error_from_message("échec de l'inscription")
    })?;

    let mut user = User::new(req.organization_id, req.display_name);
    user.roles = vec![agoravote_core::user::Role::Organizer];
    state
        .store
        .insert_user(user.clone())
        .await
        .map_err(internal_error)?;

    let account = Account::new(user.id, &email, password_hash);
    state
        .store
        .insert_account(account)
        .await
        .map_err(|e| match e {
            agoravote_store::StoreError::EmailAlreadyExists => {
                bad_request("un compte existe déjà avec cette adresse e-mail")
            }
            other => internal_error(other),
        })?;

    let (token, session) = agoravote_auth::issue_session(user.id, req.organization_id);
    state
        .store
        .insert_session(session)
        .await
        .map_err(internal_error)?;

    Ok(Json(AuthResponse { token, user }))
}

/// `POST /auth/login`.
///
/// Message d'erreur volontairement IDENTIQUE que l'email soit inconnu
/// ou que le mot de passe soit faux ("email ou mot de passe
/// incorrect") : distinguer les deux cas dans la réponse permettrait à
/// un attaquant de découvrir quelles adresses email ont un compte,
/// sans même connaître leur mot de passe (énumération de comptes).
/// C'est un principe de sécurité standard, pas un oubli de message
/// plus précis.
async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    let email = req.email.trim().to_lowercase();

    let account = state
        .store
        .get_account_by_email(&email)
        .await
        .map_err(internal_error)?
        .ok_or_else(unauthorized_login)?;

    let password_ok = agoravote_auth::verify_password(&req.password, &account.password_hash)
        .map_err(|_| unauthorized_login())?;
    if !password_ok {
        return Err(unauthorized_login());
    }

    let user = state
        .store
        .get_user(account.user_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            tracing::error!(
                user_id = %account.user_id,
                "compte trouvé mais utilisateur associé introuvable (incohérence de données)"
            );
            internal_error_from_message("erreur interne lors de la connexion")
        })?;

    let (token, session) = agoravote_auth::issue_session(user.id, user.organization_id);
    state
        .store
        .insert_session(session)
        .await
        .map_err(internal_error)?;

    Ok(Json(AuthResponse { token, user }))
}

fn unauthorized_login() -> (StatusCode, Json<ErrorResponse>) {
    tracing::warn!(
        statut = 401,
        "connexion refusée : email ou mot de passe incorrect"
    );
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse::new("email ou mot de passe incorrect")),
    )
}

/// `GET /auth/me` — renvoie l'utilisateur associé au jeton présenté.
///
/// Comble un manque découvert en vérifiant le frontend dans un vrai
/// navigateur (cf. docs/DEVLOG.md, itération 6) : sans cette route,
/// `AuthContext` ne pouvait retrouver `user` qu'à l'instant précis
/// d'un `login`/`register` — un simple rechargement de page (jeton
/// toujours valide en `localStorage`) laissait `user` à `null` alors
/// que `status` restait "authenticated", d'où un en-tête qui
/// n'affichait plus ni les liens connectés ni les liens de connexion,
/// et un tableau de bord dont "Créer la campagne" échouait
/// silencieusement (`if (!user) return;`). `AuthUser` a déjà validé le
/// jeton ; cette route ne fait que renvoyer l'utilisateur qu'il
/// désigne, sans lecture supplémentaire en base.
async fn me(AuthUser(user): AuthUser) -> Json<agoravote_core::User> {
    Json(user)
}

/// `POST /auth/logout` — supprime la session correspondant au jeton
/// présenté. Exige un jeton valide (on ne peut pas se déconnecter
/// "au nom de personne") mais son propre rôle n'a pas d'importance :
/// tout utilisateur authentifié peut mettre fin à sa propre session.
async fn logout(
    AuthUser(_user): AuthUser,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // `AuthUser` a déjà validé le jeton ; on le relit ici uniquement
    // pour recalculer son empreinte et supprimer LA bonne session
    // (`AuthUser` ne conserve pas le jeton en clair une fois résolu,
    // par cohérence avec le principe "jamais stocké au-delà du
    // nécessaire" — cf. `agoravote_core::auth::Session`).
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(unauthorized_login)?;

    state
        .store
        .delete_session(&agoravote_auth::hash_token(token))
        .await
        .map_err(internal_error)?;

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /auth/g1/challenge` — première moitié du protocole de
/// connexion Ğ1v2 optionnelle (addendum v0.3, cf.
/// `docs/G1_INTEGRATION.md` §4). Public, sans corps de requête : ne
/// fait que générer un défi frais, à signer côté client dans son
/// portefeuille (Cesium², Ğecko...) — cf. `agoravote_g1::Challenge`
/// pour le détail du mécanisme sans état côté serveur.
async fn g1_challenge() -> Json<G1ChallengeResponse> {
    Json(agoravote_g1::Challenge::generate().into())
}

/// `POST /auth/g1/verify` — seconde moitié du protocole : vérifie la
/// signature reçue, puis retrouve ou provisionne le `User` associé à
/// cette clé publique et émet une session — même mécanisme que
/// `register`/`login` ci-dessus.
///
/// ## Ce qui est vérifié ici, et ce qui ne l'est PAS
///
/// Cette route prouve uniquement la **possession de la clé privée**
/// correspondant à `public_key_hex` (signature sr25519 valide sur un
/// défi frais). Elle ne vérifie PAS l'appartenance à la toile de
/// confiance Ğ1 (`agoravote_g1::chain::check_membership`, feature
/// `chain-query`) : cette vérification réseau contre un vrai nœud
/// Ğ1v2 n'est pas câblée ici, faute de pouvoir la tester de bout en
/// bout dans l'environnement de développement actuel — cf.
/// `crates/agoravote-g1/README.md` et `docs/G1_INTEGRATION.md` §5.
/// Ne JAMAIS présenter à l'utilisateur cette connexion comme une
/// preuve de membre de la toile de confiance : ce sont deux garanties
/// distinctes (cf. le frontend, qui doit refléter cette nuance).
async fn g1_verify(
    State(state): State<AppState>,
    Json(req): Json<G1VerifyRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Le défi porte lui-même son horodatage d'expiration (cf. doc de
    // `Challenge::verify_freshness`) : pas besoin que le serveur l'ait
    // mémorisé entre l'émission et cette vérification.
    agoravote_g1::Challenge::verify_freshness(&req.message, chrono::Utc::now())
        .map_err(|_| bad_request("défi Ğ1 expiré ou invalide, redemandez-en un nouveau"))?;

    let public_key_bytes = agoravote_g1::signature::decode_hex(&req.public_key_hex)
        .map_err(|_| bad_request("clé publique Ğ1 invalide"))?;
    let signature_bytes = agoravote_g1::signature::decode_hex(&req.signature_hex)
        .map_err(|_| bad_request("signature Ğ1 invalide"))?;

    // `verify_signature` distingue une entrée malformée (`Err`, ex :
    // mauvaise longueur) d'une signature simplement invalide
    // (`Ok(false)`, ex : mauvaise clé ou tentative d'usurpation) — cf.
    // sa doc. Les deux cas se traduisent ici par un refus, mais avec
    // un log différent (400 vs 401) pour l'observabilité.
    let signature_valid =
        agoravote_g1::verify_signature(&public_key_bytes, req.message.as_bytes(), &signature_bytes)
            .map_err(|_| bad_request("format de clé ou de signature Ğ1 invalide"))?;
    if !signature_valid {
        return Err(unauthorized_g1());
    }

    // Ré-encode la clé publique à partir des octets vérifiés (plutôt
    // que de réutiliser `req.public_key_hex` telle quelle) pour que la
    // valeur stockée/recherchée soit toujours sous une forme canonique
    // (minuscules, sans préfixe `0x`), quelle que soit la façon dont le
    // client l'a formatée — sans quoi la même clé pourrait être
    // enregistrée deux fois sous deux casses différentes.
    let public_key_hex = encode_hex(&public_key_bytes);

    let user = match state
        .store
        .get_g1_link_by_public_key(&public_key_hex)
        .await
        .map_err(internal_error)?
    {
        Some(link) => state
            .store
            .get_user(link.user_id)
            .await
            .map_err(internal_error)?
            .ok_or_else(|| {
                tracing::error!(
                    user_id = %link.user_id,
                    "lien Ğ1 trouvé mais utilisateur associé introuvable (incohérence de données)"
                );
                internal_error_from_message("erreur interne lors de la connexion")
            })?,
        None => {
            // Première preuve de possession réussie pour cette clé :
            // provisionne un nouveau `User` (rôle `Voter` par défaut,
            // cf. `User::new`) dans l'organisation indiquée par le
            // client, et le lie à cette clé publique. Cf. doc de
            // `G1VerifyRequest` : jamais de `voter_id` fourni par le
            // client accepté tel quel — cette identité découle
            // uniquement de la preuve cryptographique vérifiée
            // ci-dessus.
            let display_name = format!("Participant Ğ1 {}", &public_key_hex[..8]);
            let user = User::new(req.organization_id, display_name);
            state
                .store
                .insert_user(user.clone())
                .await
                .map_err(internal_error)?;

            let link = G1Link::new(user.id, public_key_hex.clone());
            state
                .store
                .insert_g1_link(link)
                .await
                .map_err(|e| match e {
                    // Fenêtre de course entre la lecture ci-dessus et
                    // cette insertion : deux vérifications concurrentes
                    // pour la même clé, toutes deux signature valide.
                    // La contrainte `UNIQUE` en base tranche ; le
                    // perdant reçoit un 401 plutôt qu'un 500, ce n'est
                    // pas une panne (cf. `unauthorized_g1`).
                    agoravote_store::StoreError::G1PublicKeyAlreadyLinked => unauthorized_g1(),
                    other => internal_error(other),
                })?;

            user
        }
    };

    let (token, session) = agoravote_auth::issue_session(user.id, user.organization_id);
    state
        .store
        .insert_session(session)
        .await
        .map_err(internal_error)?;

    Ok(Json(AuthResponse { token, user }))
}

/// Encode des octets en hexadécimal minuscule — même motif que
/// `agoravote_auth::session::generate_token` (pas de dépendance
/// supplémentaire pour un besoin aussi simple).
fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut acc, b| {
            let _ = write!(acc, "{b:02x}");
            acc
        })
}

/// Construit une réponse `401` pour un échec de vérification Ğ1 **et**
/// journalise l'événement — même logique que [`unauthorized_login`],
/// voir sa documentation. Message volontairement générique : ne pas
/// distinguer "signature invalide" de "clé déjà liée à un autre
/// compte" évite de confirmer à un attaquant qu'une clé publique
/// donnée est déjà enregistrée sur la plateforme.
fn unauthorized_g1() -> (StatusCode, Json<ErrorResponse>) {
    tracing::warn!(statut = 401, "connexion Ğ1 refusée");
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse::new(
            "preuve de possession de clé Ğ1 invalide",
        )),
    )
}

async fn create_campaign(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateCampaignRequest>,
) -> Result<Json<Campaign>, (StatusCode, Json<ErrorResponse>)> {
    require_organizer(&user)?;
    let campaign = Campaign::new_draft(req.organization_id, req.title);
    state
        .store
        .insert_campaign(campaign.clone())
        .await
        .map_err(internal_error)?;
    Ok(Json(campaign))
}

async fn get_campaign(
    State(state): State<AppState>,
    Path(campaign_id): Path<Id>,
) -> Result<Json<Campaign>, (StatusCode, Json<ErrorResponse>)> {
    state
        .store
        .get_campaign(campaign_id)
        .await
        .map_err(internal_error)?
        .map(Json)
        .ok_or_else(|| not_found("campagne introuvable"))
}

/// `POST /campaigns/:id/form` — écran "03. Éditeur de formulaire".
///
/// Crée le formulaire de la campagne et l'y associe. Refuse si la
/// campagne n'existe pas, ou si un formulaire existe déjà (le MVP ne
/// gère pas encore l'édition d'un formulaire existant — cf. §21,
/// travaux restants). Réservé aux rôles `Organizer`/`Admin` (§13).
async fn create_form(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(campaign_id): Path<Id>,
    Json(req): Json<CreateFormRequest>,
) -> Result<Json<Form>, (StatusCode, Json<ErrorResponse>)> {
    require_organizer(&user)?;

    let mut campaign = state
        .store
        .get_campaign(campaign_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("campagne introuvable"))?;

    if campaign.form_id.is_some() {
        return Err(bad_request("un formulaire existe déjà pour cette campagne"));
    }

    let mut form = Form::new(campaign_id);
    for q in req.questions {
        form.add_question(q.prompt, q.question_type, q.required);
    }

    campaign.form_id = Some(form.id);
    state
        .store
        .insert_form(form.clone())
        .await
        .map_err(internal_error)?;
    state
        .store
        .insert_campaign(campaign)
        .await
        .map_err(internal_error)?;

    Ok(Json(form))
}

/// `GET /campaigns/:id/form` — public, symétrique de `create_form` :
/// nécessaire pour que le citoyen (écran "09. Vote citoyen") et
/// l'organisateur (écran "03" en relecture) puissent afficher les
/// questions avant de voter/publier, pas seulement les créer. Absent
/// jusqu'ici — cf. `docs/openapi.yaml` et le frontend qui en a
/// révélé le manque.
async fn get_form(
    State(state): State<AppState>,
    Path(campaign_id): Path<Id>,
) -> Result<Json<Form>, (StatusCode, Json<ErrorResponse>)> {
    state
        .store
        .form_for_campaign(campaign_id)
        .await
        .map_err(internal_error)?
        .map(Json)
        .ok_or_else(|| not_found("aucun formulaire pour cette campagne"))
}

/// `POST /campaigns/:id/publish` — cf. `Campaign::publish` (§18,
/// détection des erreurs de configuration critiques avant publication).
/// Réservé aux rôles `Organizer`/`Admin` (§13).
async fn publish_campaign(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(campaign_id): Path<Id>,
) -> Result<Json<Campaign>, (StatusCode, Json<ErrorResponse>)> {
    require_organizer(&user)?;

    let mut error = None;
    let updated = state
        .store
        .update_campaign(campaign_id, |c| {
            if let Err(e) = c.publish() {
                error = Some(e.to_string());
            }
        })
        .await
        .map_err(internal_error)?;

    match (updated, error) {
        (None, _) => Err(not_found("campagne introuvable")),
        (Some(_), Some(msg)) => Err(bad_request(&msg)),
        (Some(campaign), None) => Ok(Json(campaign)),
    }
}

/// Réservé aux rôles `Organizer`/`Admin` (§13).
async fn close_campaign(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(campaign_id): Path<Id>,
) -> Result<Json<Campaign>, (StatusCode, Json<ErrorResponse>)> {
    require_organizer(&user)?;

    let mut error = None;
    let updated = state
        .store
        .update_campaign(campaign_id, |c| {
            if let Err(e) = c.close() {
                error = Some(e.to_string());
            }
        })
        .await
        .map_err(internal_error)?;

    match (updated, error) {
        (None, _) => Err(not_found("campagne introuvable")),
        (Some(_), Some(msg)) => Err(bad_request(&msg)),
        (Some(campaign), None) => Ok(Json(campaign)),
    }
}

/// `POST /campaigns/:id/questions/:qid/ballots` — écran "09. Vote
/// citoyen". N'accepte un bulletin que si la campagne est publiée
/// (`Campaign::accepts_ballots`) — cf. §5.1, un bulletin ne doit
/// jamais pouvoir être ajouté après clôture.
///
/// Authentification optionnelle (§13) : un scrutin peut être anonyme
/// (aucun jeton fourni, comportement historique inchangé — le
/// `voter_id` du corps de requête, s'il est fourni, est alors accepté
/// tel quel, sans preuve) ou nominatif. Quand un jeton VALIDE est
/// fourni, l'identité qu'il porte prévaut TOUJOURS sur un `voter_id`
/// éventuellement présent dans le corps de la requête — un client ne
/// doit jamais pouvoir se faire passer pour un autre participant en
/// falsifiant ce champ. Un utilisateur authentifié ne peut voter
/// qu'une seule fois par question : cf. la vérification ci-dessous.
async fn cast_ballot(
    OptionalAuthUser(auth_user): OptionalAuthUser,
    State(state): State<AppState>,
    Path((campaign_id, question_id)): Path<(Id, Id)>,
    Json(req): Json<CastBallotRequest>,
) -> Result<Json<Ballot>, (StatusCode, Json<ErrorResponse>)> {
    let campaign = state
        .store
        .get_campaign(campaign_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("campagne introuvable"))?;

    if !campaign.accepts_ballots() {
        return Err(bad_request(
            "cette campagne n'accepte plus de bulletins (brouillon ou clôturée)",
        ));
    }

    // cf. doc de la fonction : un jeton valide prime toujours sur le
    // `voter_id` du corps de requête, jamais l'inverse.
    let voter_id = match &auth_user {
        Some(user) => Some(user.id),
        None => req.voter_id,
    };

    if let Some(voter_id) = voter_id {
        let existing = state
            .store
            .ballots_for_question(campaign_id, question_id)
            .await
            .map_err(internal_error)?;
        if existing.iter().any(|b| b.voter_id == Some(voter_id)) {
            return Err(bad_request(
                "vous avez déjà voté pour cette question (un seul bulletin par participant identifié)",
            ));
        }
    }

    let ballot = match req.scores {
        Some(scores) if !scores.is_empty() => {
            Ballot::for_scores(campaign_id, question_id, voter_id, scores)
        }
        _ => Ballot::for_selections(campaign_id, question_id, voter_id, req.selections),
    };

    state
        .store
        .add_ballot(ballot.clone())
        .await
        .map_err(internal_error)?;
    Ok(Json(ballot))
}

/// `POST /campaigns/:id/questions/:qid/tally` — écran "06.
/// Configuration du scrutin" / déclenche le calcul consommé par
/// l'écran "11. Résultats en direct". Réservé aux rôles
/// `Organizer`/`Admin` (§13).
///
/// C'est ici que le pont entre `agoravote-core` (bulletins) et
/// `agoravote-voting` (méthode de calcul choisie par id de module) se
/// fait concrètement : on regarde le registre, on récupère le module,
/// on lui passe les bulletins bruts et les paramètres, on obtient un
/// `TallyOutcome`, qu'on enveloppe dans un `ResultSet` traçable (§5.1).
async fn tally_question(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path((campaign_id, question_id)): Path<(Id, Id)>,
    Json(req): Json<TallyRequest>,
) -> Result<Json<ResultSet>, (StatusCode, Json<ErrorResponse>)> {
    require_organizer(&user)?;

    let method = state
        .voting_methods
        .get(&req.voting_method_id)
        .ok_or_else(|| {
            bad_request(&format!(
                "méthode de vote inconnue : {}",
                req.voting_method_id
            ))
        })?;

    let ballots = state
        .store
        .ballots_for_question(campaign_id, question_id)
        .await
        .map_err(internal_error)?;

    let params = VotingParams {
        eligible_voters: req.eligible_voters,
        quorum: req.quorum,
        seats: req.seats,
    };

    let outcome = method
        .tally(&ballots, &params)
        .map_err(|e| bad_request(&e.to_string()))?;

    let result = ResultSet::new(
        campaign_id,
        question_id,
        method.id(),
        method.version(),
        outcome,
    );
    state
        .store
        .store_result(result.clone())
        .await
        .map_err(internal_error)?;

    Ok(Json(result))
}

async fn get_results(
    State(state): State<AppState>,
    Path((_campaign_id, question_id)): Path<(Id, Id)>,
) -> Result<Json<ResultSet>, (StatusCode, Json<ErrorResponse>)> {
    state
        .store
        .get_result(question_id)
        .await
        .map_err(internal_error)?
        .map(Json)
        .ok_or_else(|| not_found("aucun résultat calculé pour cette question"))
}

/// Construit une réponse `404` **et** journalise l'événement.
///
/// C'est le correctif direct au problème signalé : sans cet appel à
/// `tracing::warn!`, un `404` était renvoyé au client mais totalement
/// invisible côté serveur (ni dans les logs, ni dans aucune métrique)
/// — une "erreur silencieuse" au sens propre : le système sait qu'une
/// erreur s'est produite, mais ne le dit à personne qui surveillerait
/// les logs plutôt que les réponses HTTP individuelles.
///
/// Niveau `warn` (pas `error`) : un 404 est souvent un usage normal
/// (lien expiré, faute de frappe dans un id) plutôt qu'un
/// dysfonctionnement du serveur — cf. `docs/LOGGING.md`, tableau des
/// niveaux, pour la convention complète du projet.
fn not_found(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    tracing::warn!(motif = message, statut = 404, "ressource introuvable");
    (StatusCode::NOT_FOUND, Json(ErrorResponse::new(message)))
}

/// Construit une réponse `400` **et** journalise l'événement — même
/// logique que [`not_found`], voir sa documentation.
fn bad_request(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    tracing::warn!(motif = message, statut = 400, "requête refusée");
    (StatusCode::BAD_REQUEST, Json(ErrorResponse::new(message)))
}

/// Construit une réponse `500` **et** journalise l'erreur complète en
/// `ERROR` — utilisé pour toute panne du store (base de données
/// injoignable, contrainte violée...). Contrairement à
/// [`not_found`]/[`bad_request`], le détail technique exact (message
/// `StoreError`) n'est JAMAIS renvoyé au client : un client de l'API
/// n'a pas à connaître le schéma SQL ou le message d'erreur natif de
/// PostgreSQL, qui pourrait par ailleurs fuiter des détails
/// d'infrastructure. Le détail complet va dans les logs serveur,
/// corrélé par l'id de requête (cf. docs/LOGGING.md).
/// Variante de [`internal_error`] pour les cas où l'erreur d'origine
/// n'est pas un `StoreError` (ex : échec de hachage de mot de passe) —
/// même comportement (log `ERROR` complet côté serveur, message
/// générique côté client), signature différente.
fn internal_error_from_message(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new(message)),
    )
}

pub(crate) fn internal_error(
    err: agoravote_store::StoreError,
) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!(erreur = %err, statut = 500, "erreur de persistance");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new(
            "erreur interne : consultez les logs serveur (id de requête dans l'en-tête x-request-id)",
        )),
    )
}

#[cfg(test)]
mod tests {
    //! Tests d'intégration du routeur complet, exécutés en mémoire via
    //! `tower::ServiceExt::oneshot` (aucun port TCP réellement ouvert).
    //! Ils démontrent concrètement les mécanismes d'observabilité
    //! décrits dans `docs/LOGGING.md` — plutôt que de se contenter de
    //! les documenter en prose, on prouve qu'ils fonctionnent.

    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_router() -> Router {
        build_router(AppState::new_memory())
    }

    #[tokio::test]
    async fn request_id_est_propage_dans_la_reponse() {
        // Vérifie l'ordre des couches documenté dans `build_router` :
        // même sans en-tête `x-request-id` fourni par le client, la
        // réponse doit en contenir un (généré par `SetRequestIdLayer`,
        // renvoyé par `PropagateRequestIdLayer`). Si l'ordre des
        // `.layer(...)` était inversé par erreur dans un futur commit,
        // ce test échouerait — c'est exactement le filet de sécurité
        // qui manquait avant que ce test n'existe.
        let response = test_router()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            response.headers().contains_key("x-request-id"),
            "la réponse devrait porter un x-request-id, généré automatiquement"
        );
    }

    #[tokio::test]
    async fn panique_dans_un_handler_devient_une_reponse_500() {
        // Construit un mini-routeur isolé (pas `build_router`, qui n'a
        // aucune route paniquant volontairement — on ne veut surtout
        // pas d'une route "panique" dans l'API réelle) pour vérifier
        // que `CatchPanicLayer`, tel qu'on le configure, transforme
        // bien une panique en 500 plutôt qu'en connexion coupée sans
        // réponse. C'est la preuve que le mécanisme décrit dans
        // `docs/LOGGING.md` sous "erreurs silencieuses de type panique"
        // fonctionne réellement, pas seulement en théorie.
        async fn route_qui_panique() -> &'static str {
            panic!("panique volontaire pour le test");
        }

        let app = Router::new()
            .route("/panique", get(route_qui_panique))
            .layer(CatchPanicLayer::new());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/panique")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn campagne_introuvable_renvoie_404() {
        let random_id = Id::new_v4();
        let response = test_router()
            .oneshot(
                Request::builder()
                    .uri(format!("/campaigns/{random_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn health_repond_ok() {
        let response = test_router()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Envoie une requête JSON et renvoie `(statut, corps désérialisé)`.
    /// Factorisé car c'est le motif répété par tous les tests
    /// d'authentification ci-dessous.
    async fn post_json(
        router: &Router,
        uri: &str,
        body: serde_json::Value,
        bearer: Option<&str>,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(token) = bearer {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        let response = router
            .clone()
            .oneshot(builder.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, json)
    }

    #[tokio::test]
    async fn creer_une_campagne_sans_jeton_renvoie_401() {
        let router = test_router();
        let (status, _) = post_json(
            &router,
            "/campaigns",
            serde_json::json!({
                "organization_id": Id::new_v4(),
                "title": "Campagne non autorisée",
            }),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn inscription_puis_creation_de_campagne_fonctionne() {
        let router = test_router();
        let organization_id = Id::new_v4();

        let (status, body) = post_json(
            &router,
            "/auth/register",
            serde_json::json!({
                "organization_id": organization_id,
                "email": "organisatrice@example.test",
                "password": "un-mot-de-passe-suffisamment-long",
                "display_name": "Organisatrice Test",
            }),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let token = body["token"]
            .as_str()
            .expect("un jeton doit être renvoyé")
            .to_string();

        // Avec le jeton obtenu à l'inscription, créer une campagne
        // doit maintenant réussir.
        let (status, body) = post_json(
            &router,
            "/campaigns",
            serde_json::json!({
                "organization_id": organization_id,
                "title": "Campagne autorisée",
            }),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["title"], "Campagne autorisée");
    }

    #[tokio::test]
    async fn double_inscription_meme_email_est_refusee() {
        let router = test_router();
        let payload = serde_json::json!({
            "organization_id": Id::new_v4(),
            "email": "duplique@example.test",
            "password": "un-mot-de-passe-suffisamment-long",
            "display_name": "Premier compte",
        });

        let (status, _) = post_json(&router, "/auth/register", payload.clone(), None).await;
        assert_eq!(status, StatusCode::OK);

        // Même email (à la casse près) : doit être refusé, pas
        // silencieusement écrasé — cf. `agoravote-store`, contrainte
        // d'unicité testée aussi au niveau du store dans
        // `postgres_integration.rs`.
        let mut second = payload;
        second["email"] = serde_json::json!("DUPLIQUE@example.test");
        let (status, _) = post_json(&router, "/auth/register", second, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn connexion_avec_mauvais_mot_de_passe_est_refusee() {
        let router = test_router();
        let (_, _) = post_json(
            &router,
            "/auth/register",
            serde_json::json!({
                "organization_id": Id::new_v4(),
                "email": "test-login@example.test",
                "password": "le-bon-mot-de-passe",
                "display_name": "Test Login",
            }),
            None,
        )
        .await;

        let (status, _) = post_json(
            &router,
            "/auth/login",
            serde_json::json!({
                "email": "test-login@example.test",
                "password": "un-mauvais-mot-de-passe",
            }),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn un_participant_authentifie_ne_peut_pas_voter_deux_fois() {
        let router = test_router();
        let organization_id = Id::new_v4();

        // Une organisatrice crée et publie une campagne avec une
        // question à choix unique.
        let (_, register_body) = post_json(
            &router,
            "/auth/register",
            serde_json::json!({
                "organization_id": organization_id,
                "email": "organisatrice2@example.test",
                "password": "un-mot-de-passe-suffisamment-long",
                "display_name": "Organisatrice",
            }),
            None,
        )
        .await;
        let organizer_token = register_body["token"].as_str().unwrap().to_string();

        let (_, campaign_body) = post_json(
            &router,
            "/campaigns",
            serde_json::json!({"organization_id": organization_id, "title": "Vote unique"}),
            Some(&organizer_token),
        )
        .await;
        let campaign_id = campaign_body["id"].as_str().unwrap();

        let (_, form_body) = post_json(
            &router,
            &format!("/campaigns/{campaign_id}/form"),
            serde_json::json!({"questions": [{
                "prompt": "Test ?", "type": "single_choice", "required": true,
                "options": [{"id": "a", "labels": {"fr": "A"}}, {"id": "b", "labels": {"fr": "B"}}]
            }]}),
            Some(&organizer_token),
        )
        .await;
        let question_id = form_body["questions"][0]["id"].as_str().unwrap();

        post_json(
            &router,
            &format!("/campaigns/{campaign_id}/publish"),
            serde_json::Value::Null,
            Some(&organizer_token),
        )
        .await;

        // Un participant s'inscrit et vote une première fois : ok.
        let (_, voter_body) = post_json(
            &router,
            "/auth/register",
            serde_json::json!({
                "organization_id": organization_id,
                "email": "participant@example.test",
                "password": "un-mot-de-passe-suffisamment-long",
                "display_name": "Participant",
            }),
            None,
        )
        .await;
        let voter_token = voter_body["token"].as_str().unwrap().to_string();

        let (status, _) = post_json(
            &router,
            &format!("/campaigns/{campaign_id}/questions/{question_id}/ballots"),
            serde_json::json!({"selections": ["a"]}),
            Some(&voter_token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        // Un second vote du même participant, sur la même question,
        // doit être refusé.
        let (status, body) = post_json(
            &router,
            &format!("/campaigns/{campaign_id}/questions/{question_id}/ballots"),
            serde_json::json!({"selections": ["b"]}),
            Some(&voter_token),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("déjà voté"));
    }

    #[tokio::test]
    async fn get_form_renvoie_le_formulaire_cree() {
        let router = test_router();
        let organization_id = Id::new_v4();

        let (_, body) = post_json(
            &router,
            "/auth/register",
            serde_json::json!({
                "organization_id": organization_id,
                "email": "get-form@example.test",
                "password": "un-mot-de-passe-suffisamment-long",
                "display_name": "Test",
            }),
            None,
        )
        .await;
        let token = body["token"].as_str().unwrap().to_string();

        let (_, campaign) = post_json(
            &router,
            "/campaigns",
            serde_json::json!({ "organization_id": organization_id, "title": "Test formulaire" }),
            Some(&token),
        )
        .await;
        let campaign_id = campaign["id"].as_str().unwrap();

        post_json(
            &router,
            &format!("/campaigns/{campaign_id}/form"),
            serde_json::json!({
                "questions": [{
                    "prompt": "Une question ?",
                    "type": "single_choice",
                    "required": true,
                    "options": [{"id": "a", "labels": {"fr": "Option A"}}]
                }]
            }),
            Some(&token),
        )
        .await;

        // Lecture publique, SANS jeton : GET /campaigns/:id/form doit
        // être accessible à un citoyen non connecté (§3.2), pas
        // seulement à l'organisateur qui l'a créé.
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/campaigns/{campaign_id}/form"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let form: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(form["questions"][0]["prompt"]["fr"], "Une question ?");
    }

    #[tokio::test]
    async fn get_form_sans_formulaire_renvoie_404() {
        let router = test_router();
        let organization_id = Id::new_v4();
        let (_, body) = post_json(
            &router,
            "/auth/register",
            serde_json::json!({
                "organization_id": organization_id,
                "email": "sans-formulaire@example.test",
                "password": "un-mot-de-passe-suffisamment-long",
                "display_name": "Test",
            }),
            None,
        )
        .await;
        let token = body["token"].as_str().unwrap().to_string();
        let (_, campaign) = post_json(
            &router,
            "/campaigns",
            serde_json::json!({ "organization_id": organization_id, "title": "Sans formulaire" }),
            Some(&token),
        )
        .await;
        let campaign_id = campaign["id"].as_str().unwrap();

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/campaigns/{campaign_id}/form"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_me_renvoie_lutilisateur_du_jeton() {
        let router = test_router();
        let organization_id = Id::new_v4();
        let (_, body) = post_json(
            &router,
            "/auth/register",
            serde_json::json!({
                "organization_id": organization_id,
                "email": "auth-me@example.test",
                "password": "un-mot-de-passe-suffisamment-long",
                "display_name": "Vérifiée après rechargement",
            }),
            None,
        )
        .await;
        let token = body["token"].as_str().unwrap().to_string();

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/auth/me")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let user: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(user["display_name"], "Vérifiée après rechargement");
    }

    #[tokio::test]
    async fn get_me_sans_jeton_renvoie_401() {
        let router = test_router();
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/auth/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // --- Identité Ğ1v2 optionnelle (addendum v0.3) ---
    //
    // Ces tests signent réellement un défi avec le compte de dev
    // Substrate standard "//Alice" (cf. commentaire du dev-dependency
    // `subxt-signer` dans Cargo.toml) : de vraies signatures sr25519,
    // pas des mocks — même exigence que le reste du projet (cf.
    // CLAUDE.md).

    /// Récupère un défi frais auprès de l'API, le signe avec le compte
    /// de dev "//Alice", et renvoie de quoi appeler `/auth/g1/verify`.
    async fn get_challenge_and_sign(router: &Router) -> (String, String, String) {
        let (status, body) =
            post_json(router, "/auth/g1/challenge", serde_json::Value::Null, None).await;
        assert_eq!(status, StatusCode::OK);
        let message = body["message"].as_str().unwrap().to_string();

        let alice = subxt_signer::sr25519::dev::alice();
        let signature = alice.sign(message.as_bytes());

        (
            message,
            super::encode_hex(&alice.public_key().0),
            super::encode_hex(&signature.0),
        )
    }

    #[tokio::test]
    async fn g1_challenge_renvoie_un_defi_avec_expiration_future() {
        let router = test_router();
        let (status, body) =
            post_json(&router, "/auth/g1/challenge", serde_json::Value::Null, None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["message"].as_str().unwrap().contains("AgoraVote"));
        assert!(body["expires_at"].is_string());
    }

    #[tokio::test]
    async fn connexion_g1_avec_signature_valide_provisionne_un_utilisateur() {
        let router = test_router();
        let organization_id = Id::new_v4();
        let (message, public_key_hex, signature_hex) = get_challenge_and_sign(&router).await;

        let (status, body) = post_json(
            &router,
            "/auth/g1/verify",
            serde_json::json!({
                "organization_id": organization_id,
                "public_key_hex": public_key_hex,
                "signature_hex": signature_hex,
                "message": message,
            }),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["token"].as_str().is_some());
        let user_id = body["user"]["id"].as_str().unwrap().to_string();

        // Une seconde connexion avec la MÊME clé (nouveau défi, nouvelle
        // signature) doit retrouver le même utilisateur, pas en
        // provisionner un second.
        let (message2, public_key_hex2, signature_hex2) = get_challenge_and_sign(&router).await;
        assert_eq!(public_key_hex, public_key_hex2);
        let (status, body) = post_json(
            &router,
            "/auth/g1/verify",
            serde_json::json!({
                "organization_id": Id::new_v4(),
                "public_key_hex": public_key_hex2,
                "signature_hex": signature_hex2,
                "message": message2,
            }),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["user"]["id"].as_str().unwrap(), user_id);
    }

    #[tokio::test]
    async fn connexion_g1_avec_signature_invalide_est_refusee() {
        let router = test_router();
        let (message, public_key_hex, _) = get_challenge_and_sign(&router).await;

        // Une signature qui ne correspond pas au message (ici : celle
        // d'un tout autre message) doit être refusée avec un 401, pas
        // acceptée ni provoquer d'erreur serveur.
        let bob = subxt_signer::sr25519::dev::bob();
        let mauvaise_signature = bob.sign(b"un autre message");

        let (status, _) = post_json(
            &router,
            "/auth/g1/verify",
            serde_json::json!({
                "organization_id": Id::new_v4(),
                "public_key_hex": public_key_hex,
                "signature_hex": super::encode_hex(&mauvaise_signature.0),
                "message": message,
            }),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn connexion_g1_avec_defi_falsifie_est_refusee() {
        let router = test_router();
        // Un message qui n'a jamais été émis par `/auth/g1/challenge`
        // (donc jamais mémorisé nulle part, cf. protocole sans état) :
        // `verify_freshness` doit le rejeter avant même de vérifier la
        // signature.
        let message = "message jamais emis par le serveur, sans marqueur EXP".to_string();
        let alice = subxt_signer::sr25519::dev::alice();
        let signature = alice.sign(message.as_bytes());

        let (status, _) = post_json(
            &router,
            "/auth/g1/verify",
            serde_json::json!({
                "organization_id": Id::new_v4(),
                "public_key_hex": super::encode_hex(&alice.public_key().0),
                "signature_hex": super::encode_hex(&signature.0),
                "message": message,
            }),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}
