//! Sessions — cf. cahier des charges §13 et la doc de
//! `agoravote_core::auth::Session` pour le choix "jeton opaque plutôt
//! que JWT" et "jeton stocké haché plutôt qu'en clair".

use chrono::{Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};

use agoravote_core::{Id, Session};

/// Durée de vie d'une session avant expiration automatique. 24h est un
/// compromis raisonnable pour un MVP : assez long pour ne pas gêner un
/// usage normal (une personne qui vote le soir après s'être connectée
/// le matin), assez court pour limiter la fenêtre d'exploitation d'un
/// jeton qui aurait fuité. À rendre configurable par organisation si
/// le besoin se présente (cf. §21, travaux restants).
pub const SESSION_DURATION: Duration = Duration::hours(24);

/// Émet une nouvelle session pour un utilisateur.
///
/// Renvoie le jeton **en clair** (à renvoyer au client, une seule
/// fois, dans la réponse de connexion) et la [`Session`] à persister
/// (qui ne contient, elle, que l'empreinte du jeton — cf.
/// [`hash_token`]).
pub fn issue_session(user_id: Id, organization_id: Id) -> (String, Session) {
    let token = generate_token();
    let now = Utc::now();
    let session = Session {
        token_hash: hash_token(&token),
        user_id,
        organization_id,
        created_at: now,
        expires_at: now + SESSION_DURATION,
    };
    (token, session)
}

/// Génère un jeton aléatoire de 256 bits, encodé en hexadécimal (64
/// caractères). 256 bits est très largement suffisant pour rendre une
/// attaque par force brute non pertinente (bien au-delà de ce qu'un
/// attaquant pourrait tenter avant l'expiration de la session) ; un
/// encodage hexadécimal simple évite d'introduire une dépendance
/// supplémentaire (base64) pour un gain marginal de compacité.
fn generate_token() -> String {
    use std::fmt::Write;

    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().fold(String::with_capacity(64), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

/// Calcule l'empreinte SHA-256 d'un jeton — utilisée à la fois pour
/// stocker une nouvelle session ([`issue_session`]) et pour vérifier
/// un jeton présenté par un client (recalculer son empreinte et
/// comparer en base, jamais stocker/comparer le jeton en clair).
///
/// SHA-256 (rapide) plutôt qu'Argon2 (lent, cf. `password.rs`) : un
/// jeton de session a déjà 256 bits d'entropie aléatoire — contrairement
/// à un mot de passe choisi par un humain, il n'y a aucun risque
/// d'attaque par dictionnaire à ralentir. L'empreinte ici protège
/// seulement contre une fuite de la base de données, pas contre une
/// recherche exhaustive de jetons plausibles.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deux_jetons_generes_sont_differents() {
        let (token1, _) = issue_session(Id::new_v4(), Id::new_v4());
        let (token2, _) = issue_session(Id::new_v4(), Id::new_v4());
        assert_ne!(token1, token2);
    }

    #[test]
    fn le_jeton_en_clair_ne_correspond_pas_a_lempreinte_stockee() {
        let (token, session) = issue_session(Id::new_v4(), Id::new_v4());
        assert_ne!(token, session.token_hash);
    }

    #[test]
    fn hash_token_est_deterministe_pour_verifier_une_session() {
        let (token, session) = issue_session(Id::new_v4(), Id::new_v4());
        // C'est exactement l'opération que fera le serveur à chaque
        // requête authentifiée : rehacher le jeton présenté et
        // comparer à l'empreinte stockée.
        assert_eq!(hash_token(&token), session.token_hash);
    }

    #[test]
    fn la_session_emise_expire_dans_le_futur_de_la_duree_prevue() {
        let (_, session) = issue_session(Id::new_v4(), Id::new_v4());
        assert!(!session.is_expired(Utc::now()));
        assert!(session.is_expired(Utc::now() + SESSION_DURATION + Duration::seconds(1)));
    }
}
