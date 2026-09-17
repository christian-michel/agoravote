//! `VotingMethodRegistry` — met à disposition les méthodes de vote
//! natives sous une forme interrogeable par id.
//!
//! Cf. cahier des charges §6 : chaque module a « une identité stable,
//! une version et des compatibilités déclarées ». Ce registre est la
//! première brique du futur "registre de modules communautaires"
//! (§15.1, phase 5 de la roadmap §16) : pour l'instant il ne contient
//! que les modules natifs compilés dans ce crate, mais son API
//! (`get`, `manifests`) est celle qu'un registre mixte
//! natif+WASM (§6.1) devra continuer à exposer.

use std::collections::HashMap;
use std::sync::Arc;

use agoravote_core::module::ModuleManifest;
use agoravote_core::voting_method::VotingMethod;

use crate::{Approval, MajorityJudgment, MajoritySimple, ScoreVoting};

/// Registre en mémoire des méthodes de vote disponibles, indexées par
/// leur identifiant de module (ex: "voting.majority").
///
/// `Arc<dyn VotingMethod>` plutôt que `Box<dyn VotingMethod>` : le
/// registre doit pouvoir être partagé (cloné à moindre coût) entre
/// plusieurs requêtes HTTP concurrentes dans `agoravote-api`, sans
/// dupliquer les modules eux-mêmes (qui sont sans état, cf. doc de
/// [`MajoritySimple`]).
#[derive(Clone)]
pub struct VotingMethodRegistry {
    methods: HashMap<&'static str, Arc<dyn VotingMethod>>,
}

impl VotingMethodRegistry {
    /// Construit le registre avec le catalogue natif complet
    /// (§15, périmètre MVP). Pour ajouter une méthode de vote native,
    /// il suffit de l'insérer ici — c'est le seul endroit du crate qui
    /// doit être modifié pour "brancher" un nouveau module dans le
    /// reste du système.
    pub fn with_builtin_methods() -> Self {
        let mut methods: HashMap<&'static str, Arc<dyn VotingMethod>> = HashMap::new();

        let majority = Arc::new(MajoritySimple);
        methods.insert(majority.id(), majority);

        let approval = Arc::new(Approval);
        methods.insert(approval.id(), approval);

        let score = Arc::new(ScoreVoting);
        methods.insert(score.id(), score);

        let majority_judgment = Arc::new(MajorityJudgment);
        methods.insert(majority_judgment.id(), majority_judgment);

        Self { methods }
    }

    /// Recherche une méthode par son identifiant de module.
    pub fn get(&self, id: &str) -> Option<Arc<dyn VotingMethod>> {
        self.methods.get(id).cloned()
    }

    /// Liste les manifestes de toutes les méthodes enregistrées —
    /// utilisé par l'écran "16. Modules" et par l'éditeur de scrutin
    /// (écran "06. Configuration du scrutin", §10.1) pour proposer le
    /// catalogue disponible.
    pub fn manifests(&self) -> Vec<ModuleManifest> {
        let mut manifests: Vec<ModuleManifest> =
            self.methods.values().map(|m| m.manifest()).collect();
        manifests.sort_by(|a, b| a.id.cmp(&b.id));
        manifests
    }
}

impl Default for VotingMethodRegistry {
    fn default() -> Self {
        Self::with_builtin_methods()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_registre_expose_les_quatre_methodes_natives() {
        let registry = VotingMethodRegistry::with_builtin_methods();
        assert!(registry.get("voting.majority").is_some());
        assert!(registry.get("voting.approval").is_some());
        assert!(registry.get("voting.score").is_some());
        assert!(registry.get("voting.majority_judgment").is_some());
        assert!(registry.get("voting.inconnu").is_none());
        assert_eq!(registry.manifests().len(), 4);
    }
}
