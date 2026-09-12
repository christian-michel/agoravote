//! # agoravote-core
//!
//! Ce crate contient le **modèle de données conceptuel** et les
//! **contrats de module** d'AgoraVote, tels que définis dans le cahier
//! des charges v0.2 :
//!
//! - §5 « Modèle de données conceptuel » → [`organization`], [`user`],
//!   [`campaign`], [`form`], [`ballot`], [`result`], [`audit`].
//! - §6 « Architecture des modules / plugins » → [`voting_method`]
//!   (le contrat `VotingMethod` et son manifeste).
//!
//! ## Pourquoi un crate séparé ?
//!
//! Le cahier des charges pose une exigence d'architecture forte (§4) :
//! *« un changement de visualisation ne doit jamais modifier le
//! résultat électoral ; un changement de méthode de vote doit produire
//! un nouveau résultat calculé à partir des mêmes données »*.
//!
//! Pour que cette règle soit vérifiable et non pas seulement déclarée,
//! on la traduit en frontière de compilation : `agoravote-core` ne
//! dépend d'aucune méthode de vote concrète, d'aucun moteur
//! statistique et d'aucune visualisation. Il ne connaît que des
//! **traits** (contrats) que les autres crates (`agoravote-voting`,
//! `agoravote-stats`, et demain les modules WASM) doivent respecter.
//!
//! Ainsi, ni le noyau ni un module de vote ne peuvent accidentellement
//! dépendre d'un détail d'implémentation d'un autre module : le seul
//! point de contact est le contrat `VotingMethod`.
//!
//! ## Ce qui n'est PAS encore ici (cf. §21 « travaux restants »)
//!
//! - Le stockage (PostgreSQL) : ce crate ne fait aucune I/O, c'est du
//!   modèle pur. Le stockage viendra dans un crate `agoravote-store`
//!   séparé (non commencé).
//! - Le chargement dynamique de modules WASM (cf. §6.1) : pour l'instant
//!   seuls les modules natifs (compilés) existent, via `agoravote-voting`.
//! - La cryptographie de vote vérifiable (§13, inspirée de Belenios/Helios) :
//!   hors périmètre MVP (§15.1).

pub mod audit;
pub mod auth;
pub mod ballot;
pub mod campaign;
pub mod form;
pub mod module;
pub mod organization;
pub mod result;
pub mod user;
pub mod voting_method;

// Ré-exports pratiques : `use agoravote_core::Campaign` plutôt que
// `use agoravote_core::campaign::Campaign`. On garde les modules
// publics malgré tout, pour la documentation (`cargo doc`) et pour
// les usages qui veulent le chemin complet.
pub use audit::AuditEvent;
pub use auth::{Account, Session};
pub use ballot::Ballot;
pub use campaign::{Campaign, CampaignStatus};
pub use form::{Form, Question, QuestionType};
pub use module::{Module, ModuleKind, ModuleManifest};
pub use organization::Organization;
pub use result::ResultSet;
pub use user::{Role, User};
pub use voting_method::{TallyOutcome, VotingError, VotingMethod, VotingParams};

/// Identifiant unique utilisé pour toutes les entités du modèle.
///
/// On centralise le choix du type d'identifiant ici : si le projet
/// décide un jour de changer de stratégie (ULID plutôt qu'UUID v4 par
/// exemple, pour des identifiants triables), un seul alias à modifier.
pub type Id = uuid::Uuid;
