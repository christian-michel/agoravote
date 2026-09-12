//! # agoravote-auth
//!
//! Hachage de mot de passe et gestion des jetons de session pour
//! AgoraVote — cf. cahier des charges §13.
//!
//! Ce crate ne fait AUCUNE I/O : il ne sait ni lire ni écrire un
//! compte ou une session (c'est le rôle de `agoravote-store`), il ne
//! sait que transformer un mot de passe en empreinte vérifiable et
//! produire/valider des jetons. Même principe de séparation que
//! `agoravote-voting` (calcul pur) vis-à-vis de `agoravote-store`
//! (persistance) — cf. `docs/ARCHITECTURE.md`.
//!
//! ## Ce qui est fait
//!
//! - [`password::hash_password`] / [`password::verify_password`] —
//!   Argon2id via le crate `argon2`, format PHC auto-descriptif.
//! - [`session::issue_session`] — génère un jeton aléatoire (256 bits)
//!   et l'empreinte SHA-256 à stocker ; [`session::hash_token`] permet
//!   de recalculer cette empreinte à la volée pour vérifier un jeton
//!   présenté par un client.
//!
//! ## Ce qui n'est PAS fait (cf. cahier des charges §21, "hors MVP")
//!
//! - MFA (§13 la mentionne comme "optionnel selon contexte").
//! - Réinitialisation de mot de passe par email (nécessiterait un
//!   service d'envoi d'email, hors périmètre actuel).
//! - Limitation de tentatives de connexion (rate limiting applicatif,
//!   au-delà du timeout HTTP générique déjà en place — cf.
//!   `docs/SECURITY.md`).

pub mod password;
pub mod session;

pub use password::{hash_password, verify_password, PasswordError};
pub use session::{hash_token, issue_session, SESSION_DURATION};
