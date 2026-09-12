//! Erreurs communes à `agoravote-g1`.

#[derive(Debug, thiserror::Error)]
pub enum G1Error {
    #[error("format de clé publique invalide (attendu : 32 octets)")]
    InvalidPublicKey,

    #[error("format de signature invalide (attendu : 64 octets, ed25519)")]
    InvalidSignature,

    #[error("le défi présenté a expiré, redemandez-en un nouveau")]
    ChallengeExpired,

    #[error("la signature ne correspond pas au défi et à la clé publique fournis")]
    SignatureMismatch,

    #[cfg(feature = "chain-query")]
    #[error("erreur de connexion ou de requête au nœud Ğ1v2")]
    Chain(String),
}
