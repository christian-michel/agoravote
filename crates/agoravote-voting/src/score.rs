//! Module `voting.score` — vote par score.
//!
//! Cf. cahier des charges §8.2 : « Vote par score : le participant
//! attribue une note à une ou plusieurs options. »
//!
//! Contrairement à `voting.majority` et `voting.approval`, ce module
//! lit `Ballot::scores` plutôt que `Ballot::selections` : c'est un
//! exemple concret de la raison pour laquelle [`Ballot`] porte
//! plusieurs champs de données optionnels (cf. doc de
//! `agoravote_core::ballot`). Le résultat de chaque option est sa
//! note moyenne sur l'ensemble des bulletins qui l'ont notée
//! (un bulletin peut noter un sous-ensemble seulement des options).

use std::collections::HashMap;

use agoravote_core::ballot::Ballot;
use agoravote_core::module::{Module, ModuleKind, ModuleManifest};
use agoravote_core::voting_method::{TallyOutcome, VotingError, VotingMethod, VotingParams};

#[derive(Debug, Default, Clone, Copy)]
pub struct ScoreVoting;

impl Module for ScoreVoting {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest {
            id: self.id().to_string(),
            version: self.version().to_string(),
            kind: ModuleKind::VotingMethod,
            inputs: vec!["score".to_string()],
            parameters: vec!["seats".to_string()],
            outputs: vec!["ranking".to_string(), "average_scores".to_string()],
            compatible_visualizations: vec!["bar".to_string(), "table".to_string()],
        }
    }
}

impl VotingMethod for ScoreVoting {
    fn id(&self) -> &'static str {
        "voting.score"
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

        // On accumule séparément la somme des notes et le nombre de
        // notes reçues par option, car deux options peuvent avoir été
        // notées par des nombres de participants différents (un
        // bulletin peut ne noter qu'un sous-ensemble des options).
        let mut sums: HashMap<String, f64> = HashMap::new();
        let mut counts: HashMap<String, f64> = HashMap::new();
        let mut valid_ballots: u64 = 0;

        for ballot in ballots {
            let Some(scores) = &ballot.scores else {
                continue;
            };
            if scores.is_empty() {
                continue;
            }
            valid_ballots += 1;
            for (option, score) in scores {
                *sums.entry(option.clone()).or_insert(0.0) += score;
                *counts.entry(option.clone()).or_insert(0.0) += 1.0;
            }
        }

        // La "percentage" ici représente la note moyenne obtenue par
        // chaque option (échelle propre au module, pas nécessairement
        // 0-100 : documenté dans `metadata.score_scale_hint`).
        let averages: HashMap<String, f64> = sums
            .iter()
            .map(|(option, sum)| {
                let n = counts.get(option).copied().unwrap_or(1.0);
                (option.clone(), sum / n)
            })
            .collect();

        let mut ranked: Vec<(&String, &f64)> = averages.iter().collect();
        ranked.sort_by(|a, b| b.1.total_cmp(a.1).then_with(|| a.0.cmp(b.0)));
        let winners: Vec<String> = ranked
            .iter()
            .take(params.seats.max(1) as usize)
            .map(|(option, _)| (*option).clone())
            .collect();

        let mut metadata = HashMap::new();
        metadata.insert(
            "score_scale_hint".to_string(),
            serde_json::json!("dépend de l'échelle de la question ; non normalisé par ce module"),
        );

        Ok(TallyOutcome {
            total_ballots,
            valid_ballots,
            winners,
            percentages: averages.clone(),
            counts: averages, // pour le score, "counts" = notes moyennes (pas de comptage brut pertinent)
            quorum_met: None,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agoravote_core::Id;
    use std::collections::HashMap as StdHashMap;

    fn ballot(scores: &[(&str, f64)]) -> Ballot {
        let mut map = StdHashMap::new();
        for (option, score) in scores {
            map.insert(option.to_string(), *score);
        }
        Ballot::for_scores(Id::new_v4(), Id::new_v4(), None, map)
    }

    #[test]
    fn calcule_la_note_moyenne_par_option() {
        let ballots = vec![
            ballot(&[("A", 5.0), ("B", 2.0)]),
            ballot(&[("A", 3.0), ("B", 4.0)]),
        ];
        let outcome = ScoreVoting
            .tally(&ballots, &VotingParams::single_winner())
            .unwrap();
        assert_eq!(outcome.percentages["A"], 4.0);
        assert_eq!(outcome.percentages["B"], 3.0);
        assert_eq!(outcome.winners, vec!["A".to_string()]);
    }

    #[test]
    fn gere_les_notations_partielles() {
        // "B" n'est noté que par un seul bulletin sur deux : sa
        // moyenne doit être calculée sur 1, pas sur 2.
        let ballots = vec![ballot(&[("A", 5.0), ("B", 1.0)]), ballot(&[("A", 5.0)])];
        let outcome = ScoreVoting
            .tally(&ballots, &VotingParams::single_winner())
            .unwrap();
        assert_eq!(outcome.percentages["A"], 5.0);
        assert_eq!(outcome.percentages["B"], 1.0);
    }
}
