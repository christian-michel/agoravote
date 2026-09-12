//! # agoravote-voting
//!
//! Catalogue des méthodes de vote **natives** (compilées dans le
//! noyau) d'AgoraVote — cf. cahier des charges §6.2 :
//! « Méthodes de vote : majorité simple, majorité absolue,
//! approbation, score, classement, Condorcet, STV, jugement
//! majoritaire, pondération ».
//!
//! Chaque méthode implémente le trait [`agoravote_core::VotingMethod`]
//! défini dans le crate `agoravote-core`. Ce crate ne dépend QUE de
//! `agoravote-core` : il ne connaît ni le stockage, ni l'API HTTP, ni
//! les statistiques descriptives. C'est la frontière qui garantit
//! qu'ajouter une méthode de vote ici ne peut pas, par erreur, changer
//! le comportement d'un autre module.
//!
//! ## Périmètre actuel (MVP, cf. §15)
//!
//! - [`majority::MajoritySimple`] — majorité simple avec quorum optionnel.
//! - [`approval::Approval`] — vote d'approbation (plusieurs choix possibles).
//! - [`score::ScoreVoting`] — vote par score (note moyenne par option).
//!
//! Les méthodes plus avancées (Condorcet/Schulze, STV, jugement
//! majoritaire — cf. §15.1 "hors MVP mais prévues dans l'architecture")
//! ne sont pas encore implémentées ; le module [`registry`] est conçu
//! pour qu'il suffise d'y enregistrer un nouveau module quand elles
//! le seront, sans toucher aux méthodes existantes.

pub mod approval;
pub mod majority;
pub mod registry;
pub mod score;

pub use approval::Approval;
pub use majority::MajoritySimple;
pub use registry::VotingMethodRegistry;
pub use score::ScoreVoting;
