//! Contrat `VotingMethod` — cf. cahier des charges §6 et §7.
//!
//! C'est le trait le plus important du projet : c'est lui qui rend le
//! catalogue de méthodes de vote extensible sans toucher au noyau
//! (§1, "principe directeur : Modularité").
//!
//! §7 distingue explicitement trois couches :
//!   - la **Question** (comment on saisit) → [`crate::form::QuestionType`]
//!   - le **Bulletin** (quelle donnée est enregistrée) → [`crate::ballot::Ballot`]
//!   - la **Méthode** (comment on détermine le résultat) → ce module
//!   - la **Règle** (quand le résultat est valide, ex: quorum) → [`VotingParams`]
//!
//! Une méthode de vote ne connaît donc que des bulletins déjà collectés
//! et des paramètres déclaratifs ; elle ne sait rien de l'UI, du
//! stockage ou de la visualisation — cf. §4, l'exigence de séparation
//! des couches.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ballot::Ballot;
use crate::module::Module;

/// Paramètres d'un scrutin, indépendants de la méthode de vote
/// elle-même — cf. §7 (couche "Règle") et §8.1 (vocabulaire électoral).
///
/// Toutes les méthodes de vote ne consomment pas tous ces paramètres :
/// [`ModuleManifest::parameters`] déclare, pour chaque module, ceux
/// qu'il prend réellement en compte. Un paramètre non pertinent pour
/// une méthode donnée est simplement ignoré par elle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VotingParams {
    /// Nombre de personnes habilitées à participer, utilisé pour
    /// calculer le taux de participation et vérifier le quorum
    /// (§8.1 : "Taux de participation" = participants / population
    /// éligible).
    pub eligible_voters: Option<u64>,

    /// Quorum requis, exprimé en fraction (0.5 = 50 %) de la
    /// population éligible. Si absent, aucun quorum n'est appliqué.
    pub quorum: Option<f64>,

    /// Nombre de sièges/gagnants à désigner (pertinent pour STV,
    /// approbation multi-gagnants...). `1` par défaut pour un scrutin
    /// à vainqueur unique.
    pub seats: u32,
}

impl VotingParams {
    pub fn single_winner() -> Self {
        Self {
            seats: 1,
            ..Default::default()
        }
    }
}

/// Résultat du dépouillement produit par une méthode de vote.
///
/// C'est la sortie brute du calcul, AVANT mise en forme pour une
/// visualisation quelconque (§9 : "Un ResultSet peut être présenté
/// sous plusieurs formes sans recalculer le scrutin"). Le crate
/// `agoravote-core` ne définit ici que le résultat d'*une* méthode ;
/// l'entité [`crate::result::ResultSet`] est l'enveloppe persistée qui
/// referme ce résultat avec ses métadonnées de traçabilité (§5.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TallyOutcome {
    /// Nombre de bulletins reçus pour cette question (avant filtrage
    /// de validité).
    pub total_ballots: u64,

    /// Nombre de bulletins retenus pour le calcul, c'est-à-dire les
    /// "suffrages exprimés" au sens du §8.1.
    pub valid_ballots: u64,

    /// Options gagnantes, dans l'ordre (plusieurs en cas d'égalité ou
    /// de scrutin à plusieurs sièges).
    pub winners: Vec<String>,

    /// Pourcentage obtenu par chaque option, calculé sur la base
    /// déclarée par la méthode (peut être les suffrages exprimés, les
    /// votants, ou les éligibles selon la méthode — c'est pourquoi
    /// cette base est aussi documentée dans `metadata`).
    pub percentages: HashMap<String, f64>,

    /// Nombre brut de voix par option (avant passage en pourcentage),
    /// utile pour l'affichage en tableau (§9, principe UX : "une vue
    /// tabulaire et/ou les valeurs exactes doivent rester
    /// accessibles").
    pub counts: HashMap<String, f64>,

    /// Le quorum était-il atteint ? `None` si aucun quorum n'était
    /// configuré pour ce scrutin.
    pub quorum_met: Option<bool>,

    /// Métadonnées libres et spécifiques à la méthode (ex: nombre de
    /// tours pour un STV, valeur de la base de calcul utilisée pour
    /// les pourcentages...). Toujours sérialisables en JSON pour rester
    /// exportables sans dépendre de l'interface (§5.1, §14).
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Erreurs pouvant survenir lors d'un dépouillement.
#[derive(Debug, thiserror::Error)]
pub enum VotingError {
    #[error("aucun bulletin à dépouiller")]
    NoBallots,

    #[error("bulletin invalide pour cette méthode : {reason}")]
    InvalidBallot { reason: String },

    #[error("paramètre requis manquant : {name}")]
    MissingParameter { name: String },

    #[error("erreur interne du module de vote : {0}")]
    Internal(String),
}

/// Contrat que doit respecter tout module de méthode de vote —
/// qu'il soit natif (compilé dans le noyau, cf. `agoravote-voting`)
/// ou, à terme, chargé dynamiquement en sandbox WASM (§6.1).
///
/// Ce trait étend [`Module`] : toute méthode de vote expose donc à la
/// fois son comportement (`tally`) et son manifeste déclaratif
/// (`manifest`, hérité de `Module`), utilisé par l'écran "16. Modules"
/// et par l'éditeur de scrutin (écran "06. Configuration du scrutin",
/// §10.1) pour savoir quelles méthodes proposer et avec quels
/// paramètres.
pub trait VotingMethod: Module {
    /// Identifiant stable du module, ex: "voting.majority".
    /// Doit être identique à `self.manifest().id`.
    fn id(&self) -> &'static str;

    /// Version sémantique du module, ex: "1.0.0".
    /// Doit être identique à `self.manifest().version`.
    fn version(&self) -> &'static str;

    /// Calcule le résultat à partir des bulletins et des paramètres
    /// du scrutin.
    ///
    /// Contrat important : cette fonction est **pure** — pas d'I/O,
    /// pas d'horodatage caché, pas d'accès réseau. À bulletins et
    /// paramètres identiques, le résultat doit être strictement
    /// identique à chaque appel. C'est ce qui rend un dépouillement
    /// rejouable et auditable (§13, §18).
    fn tally(&self, ballots: &[Ballot], params: &VotingParams)
        -> Result<TallyOutcome, VotingError>;
}
