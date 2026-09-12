//! # agoravote-g1
//!
//! Module d'identité optionnel : preuve de possession d'un compte
//! Ğ1v2 et vérification d'adhésion à la toile de confiance — cf.
//! l'addendum v0.3 du cahier des charges (« Identité décentralisée et
//! trajectoire Web3 »).
//!
//! ## ⚠️ Statut de vérification — à lire avant toute utilisation
//!
//! **Aucune ligne de ce crate n'a pu être compilée ni testée** dans
//! l'environnement de développement utilisé pour l'écrire (toolchain
//! rustc 1.75 via apt, sans accès réseau à l'infrastructure Duniter —
//! cf. `docs/G1_INTEGRATION.md` pour le détail complet des deux
//! blocages). C'est une exception assumée au reste de ce projet : pour
//! `agoravote-core`, `agoravote-voting`, `agoravote-stats`,
//! `agoravote-store` et `agoravote-auth`, chaque ligne livrée a été
//! compilée, testée, et pour beaucoup vérifiée contre un vrai
//! PostgreSQL local. Ici, le code est écrit avec le plus grand soin
//! à partir de la documentation officielle et du code source du
//! runtime Duniter v2 (noms de pallets vérifiés par lecture directe du
//! dépôt — cf. `chain.rs`), mais reste **non vérifié**.
//!
//! **Avant toute utilisation réelle** : compiler ce crate dans un
//! environnement à toolchain Rust à jour (`cargo build -p agoravote-g1
//! --features chain-query`), corriger les éventuelles erreurs d'API
//! (les signatures exactes de `subxt`/`subxt-signer` n'ont pas pu être
//! vérifiées), puis tester contre un nœud Ğ1v2 réel — d'abord `gdev`
//! (réseau de test), jamais directement `g1` (réseau de production,
//! comptes réels).
//!
//! ## Organisation
//!
//! - [`challenge`] — génération d'un défi aléatoire à signer (pur, ne
//!   nécessite aucune dépendance réseau).
//! - [`signature`] — vérification qu'une signature sr25519 correspond
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
