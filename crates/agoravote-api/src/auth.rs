//! Authentification HTTP — cf. cahier des charges §13.
//!
//! Deux extracteurs axum, utilisés comme n'importe quel autre
//! paramètre de handler (`Path`, `Json`...) :
//!
//! - [`AuthUser`] : exige un jeton valide, renvoie `401` sinon. Pour
//!   les routes d'administration (créer/publier/clôturer une
//!   campagne...).
//! - [`OptionalAuthUser`] : authentifie si un jeton est fourni, laisse
//!   passer anonymement sinon — mais renvoie quand même `401` si un
//!   jeton EST fourni mais invalide/expiré (cf. sa doc : ne jamais
//!   dégrader silencieusement une authentification ratée en accès
//!   anonyme, ce serait une "erreur silencieuse" au sens de
//!   `docs/LOGGING.md`). Pour `POST .../ballots` (§13 : un scrutin
//!   peut être anonyme ou nominatif selon la campagne).
//!
//! Les deux partagent la même logique de résolution
//! (`resolve_user_from_token`), qui vérifie dans l'ordre : présence
//! du jeton, existence de la session, non-expiration, existence de
//! l'utilisateur associé — chaque étape avec son propre message de
//! rejet (logué en `warn`, cf. `routes.rs::not_found`/`bad_request`
//! pour la même convention).

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::Json;
use axum::RequestPartsExt;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;

use agoravote_auth::hash_token;
use agoravote_core::User;

use crate::dto::ErrorResponse;
use crate::state::AppState;

// `#[async_trait::async_trait]` sur les deux impls ci-dessous : axum
// 0.7 définit `FromRequestParts::from_request_parts` avec une
// signature `async fn` compatible avec le désucrage de cette macro
// (future boxée), pas avec la syntaxe native "async fn in trait" de
// Rust 1.75+ utilisée sans elle — sans cette annotation, le
// compilateur rejette l'implémentation pour incompatibilité de durées
// de vie (E0195). Purement mécanique, sans conséquence fonctionnelle.

/// Utilisateur authentifié — cf. doc du module.
pub struct AuthUser(pub User);

/// Utilisateur authentifié si un jeton valide est fourni, `None` si
/// aucun jeton n'est fourni du tout — cf. doc du module pour la
/// nuance importante avec un jeton *invalide*.
pub struct OptionalAuthUser(pub Option<User>);

#[async_trait::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_bearer_token(parts).await?.ok_or_else(|| {
            unauthorized("authentification requise (en-tête Authorization manquant)")
        })?;
        let user = resolve_user_from_token(state, &token).await?;
        Ok(AuthUser(user))
    }
}

#[async_trait::async_trait]
impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match extract_bearer_token(parts).await? {
            None => Ok(OptionalAuthUser(None)),
            Some(token) => {
                let user = resolve_user_from_token(state, &token).await?;
                Ok(OptionalAuthUser(Some(user)))
            }
        }
    }
}

/// Lit l'en-tête `Authorization: Bearer <jeton>`. Renvoie `Ok(None)`
/// s'il est absent (cas normal pour une route publique/optionnelle),
/// `Err(401)` s'il est présent mais malformé (ni absent ni valide —
/// pas un cas ambigu à traiter comme anonyme).
async fn extract_bearer_token(
    parts: &mut Parts,
) -> Result<Option<String>, (StatusCode, Json<ErrorResponse>)> {
    match parts.extract::<TypedHeader<Authorization<Bearer>>>().await {
        Ok(TypedHeader(auth)) => Ok(Some(auth.token().to_string())),
        Err(err) if err.is_missing() => Ok(None),
        Err(_) => Err(unauthorized(
            "en-tête Authorization malformé (attendu : 'Bearer <jeton>')",
        )),
    }
}

/// Résout un jeton en clair en utilisateur authentifié : rehache le
/// jeton (cf. `agoravote_auth::hash_token`), cherche la session
/// correspondante, vérifie qu'elle n'est pas expirée, charge
/// l'utilisateur. Chaque échec a son propre message, loggé en `warn`
/// (cf. `routes.rs` pour la même convention sur les 401/404).
async fn resolve_user_from_token(
    state: &AppState,
    token: &str,
) -> Result<User, (StatusCode, Json<ErrorResponse>)> {
    let token_hash = hash_token(token);

    let session = state
        .store
        .get_session(&token_hash)
        .await
        .map_err(crate::routes::internal_error)?
        .ok_or_else(|| unauthorized("session invalide ou déjà déconnectée"))?;

    if session.is_expired(chrono::Utc::now()) {
        return Err(unauthorized("session expirée, reconnectez-vous"));
    }

    state
        .store
        .get_user(session.user_id)
        .await
        .map_err(crate::routes::internal_error)?
        .ok_or_else(|| unauthorized("utilisateur associé à la session introuvable"))
}

fn unauthorized(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    tracing::warn!(motif = message, statut = 401, "authentification refusée");
    (StatusCode::UNAUTHORIZED, Json(ErrorResponse::new(message)))
}

/// Vérifie qu'un utilisateur a le rôle `Organizer` ou `Admin` — les
/// deux seuls rôles autorisés à administrer une campagne (créer,
/// publier, clôturer, dépouiller) dans ce MVP. Cf.
/// `agoravote_core::user::Role` pour le catalogue complet des rôles.
pub fn require_organizer(user: &User) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    use agoravote_core::user::Role;
    if user.has_role(Role::Organizer) || user.has_role(Role::Admin) {
        Ok(())
    } else {
        tracing::warn!(
            user_id = %user.id,
            statut = 403,
            "action refusée : rôle organisateur ou administrateur requis"
        );
        Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "cette action nécessite le rôle organisateur ou administrateur",
            )),
        ))
    }
}
