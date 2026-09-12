//! Module `voting.approval` — vote d'approbation.
//!
//! Cf. cahier des charges §8.2 : « Vote d'approbation : le participant
//! peut approuver plusieurs options sans les classer. »
//!
//! Contrairement à la majorité simple, un bulletin avec plusieurs
//! sélections est ici parfaitement valide : c'est le principe même du
//! vote d'approbation. Chaque option sélectionnée reçoit une
//! "approbation". Le résultat classe les options par nombre
//! d'approbations et désigne comme gagnant(s) le ou les `seats`
//! meilleures options (`VotingParams::seats`), ce qui permet d'utiliser
//! cette méthode aussi bien pour élire une option unique que pour
//! sélectionner un sous-ensemble (ex: shortlist de propositions
//! retenues dans un budget participatif).

use std::collections::HashMap;

use agoravote_core::ballot::Ballot;
use agoravote_core::module::{Module, ModuleKind, ModuleManifest};
use agoravote_core::voting_method::{TallyOutcome, VotingError, VotingMethod, VotingParams};

use crate::majority::percentages_of;

#[derive(Debug, Default, Clone, Copy)]
pub struct Approval;

impl Module for Approval {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest {
            id: self.id().to_string(),
            version: self.version().to_string(),
            kind: ModuleKind::VotingMethod,
            inputs: vec!["multiple_choice".to_string()],
            parameters: vec![
                "seats".to_string(),
                "eligible_voters".to_string(),
                "quorum".to_string(),
            ],
            outputs: vec![
                "ranking".to_string(),
                "percentages".to_string(),
                "participation".to_string(),
            ],
            compatible_visualizations: vec![
                "bar".to_string(),
                "table".to_string(),
                "ranking".to_string(),
            ],
        }
    }
}

impl VotingMethod for Approval {
    fn id(&self) -> &'static str {
        "voting.approval"
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

        // En vote d'approbation, un bulletin est valide dès lors qu'il
        // comporte au moins une sélection. Un bulletin totalement vide
        // (abstention explicite sur cette question) n'est pas compté
        // dans les suffrages exprimés.
        let mut counts: HashMap<String, f64> = HashMap::new();
        let mut valid_ballots: u64 = 0;
        for ballot in ballots {
            if ballot.selections.is_empty() {
                continue;
            }
            valid_ballots += 1;
            for option in &ballot.selections {
                *counts.entry(option.clone()).or_insert(0.0) += 1.0;
            }
        }

        // Le pourcentage d'approbation d'une option se calcule
        // classiquement sur le nombre de bulletins valides (chaque
        // votant "vaut" 100 %, qu'il ait approuvé une ou dix options).
        let percentages = percentages_of(&counts, valid_ballots as f64);

        // Classement décroissant par nombre d'approbations, les
        // `seats` premières options étant désignées gagnantes.
        let mut ranked: Vec<(&String, &f64)> = counts.iter().collect();
        ranked.sort_by(|a, b| b.1.total_cmp(a.1).then_with(|| a.0.cmp(b.0)));
        let winners: Vec<String> = ranked
            .iter()
            .take(params.seats.max(1) as usize)
            .map(|(option, _)| (*option).clone())
            .collect();

        let quorum_met = match (params.quorum, params.eligible_voters) {
            (Some(quorum), Some(eligible)) if eligible > 0 => {
                Some((total_ballots as f64 / eligible as f64) >= quorum)
            }
            _ => None,
        };

        let mut metadata = HashMap::new();
        metadata.insert(
            "percentage_base".to_string(),
            serde_json::json!("valid_ballots"),
        );
        metadata.insert(
            "ranking".to_string(),
            serde_json::json!(ranked
                .iter()
                .map(|(o, c)| (o.to_string(), **c))
                .collect::<Vec<_>>()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use agoravote_core::Id;

    fn ballot(options: &[&str]) -> Ballot {
        Ballot::for_selections(
            Id::new_v4(),
            Id::new_v4(),
            None,
            options.iter().map(|s| s.to_string()).collect(),
        )
    }

    #[test]
    fn compte_une_approbation_par_option_selectionnee() {
        let ballots = vec![ballot(&["A", "B"]), ballot(&["A"]), ballot(&["B", "C"])];
        let outcome = Approval
            .tally(&ballots, &VotingParams::single_winner())
            .unwrap();
        assert_eq!(outcome.counts["A"], 2.0);
        assert_eq!(outcome.counts["B"], 2.0);
        assert_eq!(outcome.counts["C"], 1.0);
        assert_eq!(outcome.valid_ballots, 3);
    }

    #[test]
    fn peut_designer_plusieurs_gagnants_selon_seats() {
        let ballots = vec![
            ballot(&["A"]),
            ballot(&["B"]),
            ballot(&["C"]),
            ballot(&["A", "B"]),
        ];
        let params = VotingParams {
            seats: 2,
            ..Default::default()
        };
        let outcome = Approval.tally(&ballots, &params).unwrap();
        assert_eq!(outcome.winners.len(), 2);
        assert!(outcome.winners.contains(&"A".to_string()));
        assert!(outcome.winners.contains(&"B".to_string()));
    }

    #[test]
    fn ignore_les_bulletins_vides_dans_les_suffrages_exprimes() {
        let ballots = vec![ballot(&["A"]), ballot(&[])];
        let outcome = Approval
            .tally(&ballots, &VotingParams::single_winner())
            .unwrap();
        assert_eq!(outcome.total_ballots, 2);
        assert_eq!(outcome.valid_ballots, 1);
    }
}
