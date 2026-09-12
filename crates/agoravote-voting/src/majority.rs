//! Module `voting.majority` — majorité simple.
//!
//! Cf. cahier des charges §7 (exemple filé du document) et §19
//! (glossaire) : « Majorité simple : règle où l'option obtenant le
//! plus de voix l'emporte, sous réserve des règles définies. »
//!
//! Entrée attendue : des bulletins de type `single_choice`, c'est-à-dire
//! dont `Ballot::selections` contient exactement une option. Un
//! bulletin avec zéro ou plusieurs sélections est compté comme
//! présent (`total_ballots`) mais écarté des suffrages exprimés
//! (§8.1 : "vote nul").

use std::collections::HashMap;

use agoravote_core::ballot::Ballot;
use agoravote_core::module::{Module, ModuleKind, ModuleManifest};
use agoravote_core::voting_method::{TallyOutcome, VotingError, VotingMethod, VotingParams};

/// Implémentation de la méthode "majorité simple".
///
/// Type unitaire (`struct` sans champ) : la méthode n'a pas d'état
/// propre, tous ses paramètres viennent de [`VotingParams`] à chaque
/// appel de `tally`. Cela garantit la pureté exigée par le contrat
/// [`VotingMethod::tally`] (cf. doc du trait).
#[derive(Debug, Default, Clone, Copy)]
pub struct MajoritySimple;

impl Module for MajoritySimple {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest {
            id: self.id().to_string(),
            version: self.version().to_string(),
            kind: ModuleKind::VotingMethod,
            inputs: vec!["single_choice".to_string()],
            parameters: vec!["quorum".to_string(), "eligible_voters".to_string()],
            outputs: vec![
                "winner".to_string(),
                "percentages".to_string(),
                "participation".to_string(),
            ],
            compatible_visualizations: vec![
                "bar".to_string(),
                "donut".to_string(),
                "table".to_string(),
            ],
        }
    }
}

impl VotingMethod for MajoritySimple {
    fn id(&self) -> &'static str {
        "voting.majority"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn tally(
        &self,
        ballots: &[Ballot],
        params: &VotingParams,
    ) -> Result<TallyOutcome, VotingError> {
        if ballots.is_empty() {
            return Err(VotingError::NoBallots);
        }

        let total_ballots = ballots.len() as u64;

        // Suffrages exprimés = bulletins comportant exactement une
        // sélection (cf. doc de module ci-dessus). Les autres sont
        // comptés dans `total_ballots` mais pas dans `valid_ballots`,
        // ni dans les compteurs par option — cf. §8.1 "vote nul".
        let mut counts: HashMap<String, f64> = HashMap::new();
        let mut valid_ballots: u64 = 0;
        for ballot in ballots {
            // `.first()` couplé à la vérification de longueur exacte
            // (plutôt qu'une indexation `[0]`) : aucun risque de panique
            // même si cette condition venait à être modifiée par erreur
            // dans une future évolution du fichier.
            if ballot.selections.len() == 1 {
                if let Some(option) = ballot.selections.first() {
                    *counts.entry(option.clone()).or_insert(0.0) += 1.0;
                    valid_ballots += 1;
                }
            }
        }

        // Le quorum se calcule sur la base des électeurs éligibles
        // quand elle est fournie, sinon sur les bulletins reçus
        // (cf. §8.1 "Taux de participation") — mais dans ce dernier
        // cas un quorum n'a pas vraiment de sens, donc `quorum_met`
        // reste `None` si `eligible_voters` est absent.
        let quorum_met = match (params.quorum, params.eligible_voters) {
            (Some(quorum), Some(eligible)) if eligible > 0 => {
                Some((total_ballots as f64 / eligible as f64) >= quorum)
            }
            _ => None,
        };

        let percentages = percentages_of(&counts, valid_ballots as f64);
        let winners = winners_of(&counts);

        let mut metadata = HashMap::new();
        metadata.insert(
            "percentage_base".to_string(),
            serde_json::json!("valid_ballots"),
        );

        Ok(TallyOutcome {
            total_ballots,
            valid_ballots,
            winners,
            percentages,
            counts,
            quorum_met,
            metadata,
        })
    }
}

/// Calcule les pourcentages de chaque option par rapport à une base
/// donnée (suffrages exprimés, votants ou éligibles selon la méthode).
/// Fonction utilitaire partagée par plusieurs méthodes de ce crate.
pub(crate) fn percentages_of(counts: &HashMap<String, f64>, base: f64) -> HashMap<String, f64> {
    if base <= 0.0 {
        return counts.keys().map(|k| (k.clone(), 0.0)).collect();
    }
    counts
        .iter()
        .map(|(option, count)| (option.clone(), (count / base) * 100.0))
        .collect()
}

/// Détermine la ou les options gagnantes (plusieurs en cas d'égalité
/// stricte au score maximal).
pub(crate) fn winners_of(counts: &HashMap<String, f64>) -> Vec<String> {
    let max = counts.values().cloned().fold(f64::MIN, f64::max);
    if max == f64::MIN {
        return Vec::new();
    }
    let mut winners: Vec<String> = counts
        .iter()
        .filter(|(_, &v)| v == max)
        .map(|(k, _)| k.clone())
        .collect();
    winners.sort(); // ordre déterministe, indépendant de HashMap
    winners
}

#[cfg(test)]
mod tests {
    use super::*;
    use agoravote_core::Id;

    fn ballot(option: &str) -> Ballot {
        Ballot::for_selections(Id::new_v4(), Id::new_v4(), None, vec![option.to_string()])
    }

    #[test]
    fn refuse_de_depouiller_sans_bulletin() {
        let method = MajoritySimple;
        let result = method.tally(&[], &VotingParams::single_winner());
        assert!(matches!(result, Err(VotingError::NoBallots)));
    }

    #[test]
    fn designe_loption_la_plus_choisie() {
        let ballots = vec![
            ballot("A"),
            ballot("A"),
            ballot("A"),
            ballot("B"),
            ballot("B"),
            ballot("C"),
        ];
        let method = MajoritySimple;
        let outcome = method
            .tally(&ballots, &VotingParams::single_winner())
            .unwrap();

        assert_eq!(outcome.total_ballots, 6);
        assert_eq!(outcome.valid_ballots, 6);
        assert_eq!(outcome.winners, vec!["A".to_string()]);
        assert!((outcome.percentages["A"] - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn detecte_une_egalite() {
        let ballots = vec![ballot("A"), ballot("B")];
        let method = MajoritySimple;
        let outcome = method
            .tally(&ballots, &VotingParams::single_winner())
            .unwrap();
        assert_eq!(outcome.winners, vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn calcule_le_quorum_par_rapport_aux_eligibles() {
        let ballots = vec![ballot("A"), ballot("A"), ballot("B")];
        let params = VotingParams {
            eligible_voters: Some(10),
            quorum: Some(0.5),
            seats: 1,
        };
        let method = MajoritySimple;
        let outcome = method.tally(&ballots, &params).unwrap();
        // 3 bulletins / 10 éligibles = 30 % < 50 % de quorum
        assert_eq!(outcome.quorum_met, Some(false));
    }

    #[test]
    fn ecarte_les_bulletins_a_selections_multiples_des_suffrages_exprimes() {
        let mut invalid = ballot("A");
        invalid.selections.push("B".to_string()); // 2 sélections = invalide pour un choix unique
        let ballots = vec![ballot("A"), invalid];
        let method = MajoritySimple;
        let outcome = method
            .tally(&ballots, &VotingParams::single_winner())
            .unwrap();
        assert_eq!(outcome.total_ballots, 2);
        assert_eq!(outcome.valid_ballots, 1);
    }
}
