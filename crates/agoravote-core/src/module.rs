//! Contrat générique de module — cf. cahier des charges §6
//! « Architecture des modules / plugins ».
//!
//! Le cahier des charges définit un module par son manifeste :
//!
//! ```yaml
//! module:
//!   id: voting.majority
//!   version: 1.0
//!   inputs:
//!     - single_choice
//!   parameters:
//!     - quorum
//!     - abstention_policy
//!   outputs:
//!     - winner
//!     - percentages
//!     - participation
//!   compatible_visualizations:
//!     - bar
//!     - donut
//!     - table
//! ```
//!
//! [`ModuleManifest`] est la traduction directe de ce YAML en type
//! Rust. Chaque famille de module (méthode de vote, type de question,
//! statistique, visualisation, export — §6.2) implémentera son propre
//! trait spécialisé (voir [`crate::voting_method::VotingMethod`] pour
//! le premier d'entre eux), mais tous exposent un [`ModuleManifest`]
//! de façon uniforme : c'est ce qui permet à l'écran "16. Modules"
//! (§10.1) d'afficher n'importe quel module, natif ou WASM, sans
//! connaître son type concret.

use serde::{Deserialize, Serialize};

/// Famille fonctionnelle d'un module, telle que listée en §6.2.
///
/// Seule `VotingMethod` est implémentée dans le MVP actuel
/// (cf. `agoravote-voting`). Les autres variantes existent déjà dans
/// l'énumération pour que le modèle de données n'ait pas besoin de
/// changer de forme quand ces familles seront développées.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    /// Type de question (choix unique, échelle...) — §6.2.
    QuestionType,
    /// Méthode de vote (majorité, approbation, Condorcet...) — §6.2, §7.
    VotingMethod,
    /// Moteur statistique (moyenne, écart-type...) — §6.2, §8.
    Statistic,
    /// Visualisation (barres, donut...) — §6.2, §9.
    Visualization,
    /// Format d'export (CSV, JSON, XML électoral) — §6.2, §14.
    Export,
}

/// Manifeste déclaratif d'un module : identité, version, contrat
/// d'entrées/sorties, paramètres configurables et visualisations
/// compatibles.
///
/// C'est cette structure — et non le code du module — qui est
/// affichée à l'administrateur et stockée dans l'audit (§6, §18 :
/// « les modules sont identifiables et versionnés »).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    /// Identifiant stable, ex: "voting.majority". Par convention,
    /// préfixé par la famille du module ("voting.", "stat.", "viz.").
    pub id: String,

    /// Version sémantique (ex: "1.0.0"). Un `ResultSet` référence
    /// toujours cette version exacte (§5.1), jamais "la dernière
    /// version installée".
    pub version: String,

    pub kind: ModuleKind,

    /// Types d'entrée acceptés (ex: "single_choice", "ranking").
    /// Sert à filtrer, dans l'éditeur de scrutin (écran 06), les
    /// méthodes de vote compatibles avec le type de question choisi.
    pub inputs: Vec<String>,

    /// Noms des paramètres que ce module accepte (ex: "quorum",
    /// "abstention_policy"). La validation des valeurs elles-mêmes se
    /// fait côté module (cf. `VotingMethod::tally`), ce champ ne sert
    /// qu'à la découverte / documentation dans l'interface d'admin.
    pub parameters: Vec<String>,

    /// Sorties produites par le module (ex: "winner", "percentages").
    pub outputs: Vec<String>,

    /// Identifiants de visualisation compatibles avec les sorties de
    /// ce module (§9), pour ne proposer à l'administrateur que des
    /// restitutions pertinentes.
    pub compatible_visualizations: Vec<String>,
}

/// Contrat minimal commun à tout module installable, quelle que soit
/// sa famille. Chaque famille concrète (ex: [`crate::voting_method::VotingMethod`])
/// étend ce trait avec son comportement propre.
///
/// `Send + Sync` est requis car le registre de modules sera partagé
/// entre threads dans l'API HTTP (cf. `agoravote-api`).
pub trait Module: Send + Sync {
    /// Décrit le module — voir [`ModuleManifest`].
    fn manifest(&self) -> ModuleManifest;
}
