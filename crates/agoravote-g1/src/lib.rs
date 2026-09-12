//! # agoravote-g1
//!
//! Module d'identité optionnel : preuve de possession d'un compte
//! Ğ1v2 et vérification d'adhésion à la toile de confiance — cf.
//! l'addendum v0.3 du cahier des charges (« Identité décentralisée et
//! trajectoire Web3 »).
//!
//! ## ⚠️ Statut de vérification — à lire avant toute utilisation
//!
//! [`challenge`] et [`signature`] sont compilés, testés (13 tests
//! verts, `cargo test -p agoravote-g1 --features chain-query`) et
//! câblés en production dans `agoravote-api` depuis l'itération 8 (cf.
//! `docs/DEVLOG.md`). [`chain`] compile également mais **reste non
//! vérifié à l'exécution ni câblé à aucune route** : l'infrastructure
//! réseau Duniter/Ğ1 reste bloquée par la liste blanche réseau de tout
//! environnement de développement utilisé jusqu'ici (testé à nouveau,
//! toujours bloqué) — les noms de stockage qu'il interroge
//! (`IdentityIndexOf`, `Membership`) restent des hypothèses non
//! confirmées contre un nœud réel.
//!
//! **Correctif important (itération 9)** : [`signature`] vérifie des
//! signatures **ed25519**, pas sr25519 comme une version antérieure le
//! faisait par erreur — cf. l'avertissement en tête de
//! `signature.rs`. Cette correction s'appuie sur des recherches web
//! (le forum et le dépôt git de Duniter restent bloqués par la même
//! liste blanche réseau que `chain.rs`), pas sur une lecture directe
//! de la documentation officielle : à reconfirmer avant une mise en
//! production.
//!
//! **Avant toute utilisation de [`chain::check_membership`]** :
//! confirmer les noms de stockage contre un vrai nœud `gdev` (réseau
//! de test — jamais `g1` directement) depuis un environnement qui a
//! accès à l'infrastructure Duniter — cf. `crates/agoravote-g1/README.md`
//! pour la marche à suivre complète.
//!
//! ## Organisation
//!
//! - [`challenge`] — génération d'un défi aléatoire à signer (pur, ne
//!   nécessite aucune dépendance réseau).
//! - [`signature`] — vérification qu'une signature ed25519 correspond
//!   bien à une clé publique donnée pour un message donné (pur,
//!   cryptographie hors-ligne — **jamais** la phrase de 12 mots,
//!   cf. sa documentation).
//! - [`chain`] *(feature `chain-query`, désactivée par défaut)* —
//!   requête au réseau Ğ1v2 pour vérifier qu'un compte est membre actif
//!   de la toile de confiance.
//! - [`error`] — erreurs communes à tout le crate.

pub mod challenge;
pub mod error;

#[cfg(feature = "signature-verification")]
pub mod signature;

#[cfg(feature = "chain-query")]
pub mod chain;

pub use challenge::Challenge;
pub use error::G1Error;

#[cfg(feature = "signature-verification")]
pub use signature::verify_signature;
