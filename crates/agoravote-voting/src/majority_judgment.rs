//! Module `voting.majority_judgment` — jugement majoritaire.
//!
//! Cf. cahier des charges §6.2 et §15.1, qui citent explicitement le
//! « jugement majoritaire » parmi les méthodes de vote avancées
//! prévues dans l'architecture mais non encore implémentées.
//!
//! Chaque bulletin attribue une MENTION à CHAQUE option de la
//! question — pas un choix, ni une note libre : une valeur entière
//! fixe de 1 (Très défavorable) à 6 (Ne sait pas). Comme
//! `voting.score`, ce module lit `Ballot::scores` (une option peut
//! être notée par un sous-ensemble seulement des bulletins), mais
//! calcule la MÉDIANE des mentions par option plutôt que leur
//! moyenne : c'est la méthode du jugement majoritaire proprement dite
//! (réputée plus résistante aux stratégies de vote qu'un score
//! moyenné, cf. §6.2), pas une variante de `voting.score`.
//!
//! ## Traitement de "Ne sait pas" (mention 6)
//!
//! La mention 6 est neutralisée en 3 ("Sans avis") avant le calcul de
//! la médiane. Sans cette neutralisation, une option qui ne recevrait
//! QUE des "Ne sait pas" n'aurait pas de médiane définie sur un
//! ensemble de mentions "opinables" et remonterait arbitrairement en
//! tête par défaut — ce qui avantagerait injustement l'absence
//! d'opinion. Neutraliser en "Sans avis" (valeur médiane de l'échelle)
//! garantit qu'une option qui ne récolte que des "Ne sait pas" obtient
//! le résultat le plus neutre possible, jamais un avantage. La mention
//! reste comptée et visible dans la répartition détaillée
//! (`TallyOutcome::metadata`), simplement neutre pour le classement.
//!
//! ## Convention de médiane et départage
//!
//! Sur un nombre pair de mentions valides, la "médiane" retenue est
//! l'élément d'indice `len / 2` après tri croissant (la plus haute des
//! deux valeurs centrales) — une convention déterministe parmi
//! plusieurs défendables ; ce qui compte est qu'elle soit fixe et
//! documentée, pas laquelle. En cas d'égalité de médiane entre deux
//! options, le départage suit l'ordre : (1) % de mentions à ou
//! au-dessus de la médiane le plus élevé, (2) à égalité, % de mentions
//! strictement en-dessous le plus faible.

use std::collections::HashMap;

use agoravote_core::ballot::Ballot;
use agoravote_core::module::{Module, ModuleKind, ModuleManifest};
use agoravote_core::voting_method::{TallyOutcome, VotingError, VotingMethod, VotingParams};
use serde::Serialize;

/// Répartition détaillée des mentions reçues par une option — cf.
/// `TallyOutcome::metadata["mention_breakdown"]`, consommée par le
/// frontend pour dessiner la barre empilée par mentions (couleurs
/// dédiées, pas les couleurs génériques des autres méthodes).
#[derive(Debug, Clone, Serialize)]
struct MentionBreakdown {
    /// Nombre brut de bulletins par mention (clé "1" à "6").
    counts: HashMap<String, u64>,
    /// Pourcentage de bulletins valides par mention (clé "1" à "6").
    percentages: HashMap<String, f64>,
    /// Médiane calculée (1 à 5 — jamais 6, neutralisée avant tri).
    median_mention: u8,
    /// % de mentions à ou au-dessus de la médiane (sert de départage
    /// et de badge d'affichage "(≥X %)").
    percentage_at_or_above_median: f64,
    /// % de mentions strictement en-dessous de la médiane.
    percentage_below_median: f64,
}

/// Implémentation de la méthode "jugement majoritaire".
#[derive(Debug, Default, Clone, Copy)]
pub struct MajorityJudgment;

impl Module for MajorityJudgment {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest {
            id: self.id().to_string(),
            version: self.version().to_string(),
            kind: ModuleKind::VotingMethod,
            inputs: vec!["majority_judgment".to_string()],
            parameters: vec!["seats".to_string()],
            outputs: vec![
                "ranking".to_string(),
                "median_mention".to_string(),
                "mention_breakdown".to_string(),
            ],
            compatible_visualizations: vec!["stacked_bar_mentions".to_string()],
        }
    }
}

/// Résultat intermédiaire d'une option, avant tri — sépare le calcul
/// (ci-dessous) de la mise en forme finale en `TallyOutcome`.
struct OptionTally {
    median_mention: u8,
    percentage_at_or_above_median: f64,
    percentage_below_median: f64,
    breakdown: MentionBreakdown,
}

impl VotingMethod for MajorityJudgment {
    fn id(&self) -> &'static str {
        "voting.majority_judgment"
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

        // Regroupe les mentions brutes (1-6, avant neutralisation) par
        // option — une option peut ne pas être notée par tous les
        // bulletins (même logique que `voting.score`).
        let mut raw_mentions: HashMap<String, Vec<u8>> = HashMap::new();
        let mut valid_ballots: u64 = 0;

        for ballot in ballots {
            let Some(scores) = &ballot.scores else {
                continue;
            };
            if scores.is_empty() {
                continue;
            }
            valid_ballots += 1;
            for (option, mention) in scores {
                // Défensif : un bulletin malformé (hors 1-6) est ignoré
                // plutôt que de fausser le calcul — la validation
                // stricte à la soumission (cf. `agoravote-api::build_ballot`)
                // devrait déjà l'empêcher, mais ce module ne doit pas
                // faire confiance aux données qu'on lui passe (§13).
                let value = *mention;
                if !(1.0..=6.0).contains(&value) || value.fract() != 0.0 {
                    continue;
                }
                raw_mentions
                    .entry(option.clone())
                    .or_default()
                    .push(value as u8);
            }
        }

        if raw_mentions.is_empty() {
            return Err(VotingError::NoBallots);
        }

        let mut tallies: HashMap<String, OptionTally> = HashMap::new();
        for (option, raw) in &raw_mentions {
            tallies.insert(option.clone(), tally_option(raw));
        }

        // Classement : médiane la plus haute d'abord, puis départage
        // par % au-dessus, puis % en-dessous (le plus faible gagne),
        // puis id d'option pour un ordre total déterministe entre
        // options rigoureusement à égalité.
        let mut ranked: Vec<&String> = tallies.keys().collect();
        ranked.sort_by(|a, b| {
            let ta = &tallies[*a];
            let tb = &tallies[*b];
            tb.median_mention
                .cmp(&ta.median_mention)
                .then_with(|| {
                    tb.percentage_at_or_above_median
                        .total_cmp(&ta.percentage_at_or_above_median)
                })
                .then_with(|| {
                    ta.percentage_below_median
                        .total_cmp(&tb.percentage_below_median)
                })
                .then_with(|| a.cmp(b))
        });

        let winners: Vec<String> = ranked
            .iter()
            .take(params.seats.max(1) as usize)
            .map(|s| (*s).clone())
            .collect();

        let mut percentages = HashMap::new();
        let mut counts = HashMap::new();
        let mut breakdowns: HashMap<String, MentionBreakdown> = HashMap::new();
        for (option, tally) in tallies {
            percentages.insert(option.clone(), tally.percentage_at_or_above_median);
            // Comme `voting.score` (cf. sa doc), "counts" est réutilisé
            // pour porter la médiane par option plutôt qu'un comptage
            // brut : un comptage par mention seule n'aurait pas de sens
            // agrégé à ce niveau (cf. `metadata` pour le détail).
            counts.insert(option.clone(), f64::from(tally.median_mention));
            breakdowns.insert(option, tally.breakdown);
        }

        let mut metadata = HashMap::new();
        metadata.insert(
            "mention_breakdown".to_string(),
            serde_json::to_value(&breakdowns).map_err(|e| VotingError::Internal(e.to_string()))?,
        );
        metadata.insert(
            "median_convention".to_string(),
            serde_json::json!(
                "médiane à l'indice floor(n/2) après tri croissant des mentions valides ; \
                 mention 6 (\"Ne sait pas\") neutralisée en 3 (\"Sans avis\") avant tri"
            ),
        );

        Ok(TallyOutcome {
            total_ballots,
            valid_ballots,
            winners,
            percentages,
            counts,
            quorum_met: None,
            metadata,
        })
    }
}

/// Calcule médiane + départage + répartition pour UNE option, à
/// partir de ses mentions brutes (1-6, non triées).
fn tally_option(raw: &[u8]) -> OptionTally {
    let total = raw.len();

    // Répartition brute (comptage ET %), AVANT neutralisation : "Ne
    // sait pas" doit rester visible tel quel dans le détail affiché,
    // même s'il est neutralisé pour le calcul de la médiane
    // ci-dessous (cf. doc de module).
    let mut counts: HashMap<String, u64> = HashMap::new();
    for mention in raw {
        *counts.entry(mention.to_string()).or_insert(0) += 1;
    }
    let mut percentages: HashMap<String, f64> = HashMap::new();
    for value in 1..=6u8 {
        let count = counts.get(&value.to_string()).copied().unwrap_or(0);
        percentages.insert(value.to_string(), (count as f64) * 100.0 / (total as f64));
    }
    for value in 1..=6u8 {
        counts.entry(value.to_string()).or_insert(0);
    }

    // Neutralisation 6 -> 3 puis tri, pour le calcul de la médiane.
    let mut neutralized: Vec<u8> = raw.iter().map(|&v| if v == 6 { 3 } else { v }).collect();
    neutralized.sort_unstable();

    let median_index = neutralized.len() / 2;
    let median_mention = neutralized[median_index];

    let below = neutralized.iter().filter(|&&v| v < median_mention).count();
    let n = neutralized.len() as f64;
    let percentage_below_median = (below as f64) * 100.0 / n;
    let percentage_at_or_above_median = 100.0 - percentage_below_median;

    OptionTally {
        median_mention,
        percentage_at_or_above_median,
        percentage_below_median,
        breakdown: MentionBreakdown {
            counts,
            percentages,
            median_mention,
            percentage_at_or_above_median,
            percentage_below_median,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agoravote_core::Id;
    use std::collections::HashMap as StdHashMap;

    fn ballot(mentions: &[(&str, u8)]) -> Ballot {
        let mut map = StdHashMap::new();
        for (option, mention) in mentions {
            map.insert(option.to_string(), f64::from(*mention));
        }
        Ballot::for_scores(Id::new_v4(), Id::new_v4(), None, map)
    }

    #[test]
    fn calcule_la_mediane_par_option_et_classe_par_mediane_decroissante() {
        // RIC : 5 votants, mentions [1, 2, 3, 4, 5] -> médiane indice 2 = 3
        // OUIII : 5 votants, mentions [1, 2, 2, 3, 3] -> médiane indice 2 = 2
        let ballots = vec![
            ballot(&[("ric", 1), ("ouiii", 1)]),
            ballot(&[("ric", 2), ("ouiii", 2)]),
            ballot(&[("ric", 3), ("ouiii", 2)]),
            ballot(&[("ric", 4), ("ouiii", 3)]),
            ballot(&[("ric", 5), ("ouiii", 3)]),
        ];
        let outcome = MajorityJudgment
            .tally(&ballots, &VotingParams::single_winner())
            .unwrap();
        assert_eq!(outcome.counts["ric"], 3.0);
        assert_eq!(outcome.counts["ouiii"], 2.0);
        assert_eq!(outcome.winners, vec!["ric".to_string()]);
    }

    #[test]
    fn depart_les_egalites_de_mediane_par_pourcentage_au_dessus() {
        // Deux options avec médiane 3 (Sans avis), mais A a plus de
        // mentions AU-DESSUS de 3 que B -> A doit gagner.
        // A : [1, 3, 3, 4, 5] -> médiane index 2 = 3, au-dessus: 4,5 (2/5=40%)
        // B : [1, 2, 3, 3, 5] -> médiane index 2 = 3, au-dessus: 5 (1/5=20%)
        let ballots = vec![
            ballot(&[("a", 1), ("b", 1)]),
            ballot(&[("a", 3), ("b", 2)]),
            ballot(&[("a", 3), ("b", 3)]),
            ballot(&[("a", 4), ("b", 3)]),
            ballot(&[("a", 5), ("b", 5)]),
        ];
        let outcome = MajorityJudgment
            .tally(&ballots, &VotingParams::single_winner())
            .unwrap();
        assert_eq!(outcome.counts["a"], 3.0);
        assert_eq!(outcome.counts["b"], 3.0);
        assert_eq!(outcome.winners, vec!["a".to_string()]);
        assert!(outcome.percentages["a"] > outcome.percentages["b"]);
    }

    #[test]
    fn neutralise_ne_sait_pas_en_sans_avis_plutot_que_de_favoriser_labsence_davis() {
        // Option "abstenue" : uniquement des "Ne sait pas" (6) -> doit
        // obtenir la médiane neutre (3 = Sans avis), jamais remonter en
        // tête par défaut face à une option réellement "Favorable" (4).
        let ballots = vec![
            ballot(&[("abstenue", 6), ("favorable", 4)]),
            ballot(&[("abstenue", 6), ("favorable", 4)]),
            ballot(&[("abstenue", 6), ("favorable", 5)]),
        ];
        let outcome = MajorityJudgment
            .tally(&ballots, &VotingParams::single_winner())
            .unwrap();
        assert_eq!(outcome.counts["abstenue"], 3.0, "neutralisée en Sans avis");
        assert_eq!(outcome.counts["favorable"], 4.0);
        assert_eq!(
            outcome.winners,
            vec!["favorable".to_string()],
            "l'option réellement favorable doit gagner, pas celle qui n'a que des \"Ne sait pas\""
        );
    }

    #[test]
    fn conserve_ne_sait_pas_visible_dans_la_repartition_detaillee() {
        let ballots = vec![
            ballot(&[("x", 6)]),
            ballot(&[("x", 6)]),
            ballot(&[("x", 4)]),
        ];
        let outcome = MajorityJudgment
            .tally(&ballots, &VotingParams::single_winner())
            .unwrap();
        let breakdown = outcome
            .metadata
            .get("mention_breakdown")
            .expect("breakdown présent")
            .get("x")
            .expect("option x présente")
            .clone();
        assert_eq!(breakdown["counts"]["6"], 2);
        assert_eq!(breakdown["counts"]["4"], 1);
    }

    #[test]
    fn gere_les_notations_partielles_comme_voting_score() {
        // "b" n'est noté que par un seul bulletin sur deux.
        let ballots = vec![ballot(&[("a", 5), ("b", 1)]), ballot(&[("a", 5)])];
        let outcome = MajorityJudgment
            .tally(&ballots, &VotingParams::single_winner())
            .unwrap();
        assert_eq!(outcome.counts["a"], 5.0);
        assert_eq!(outcome.counts["b"], 1.0);
    }

    #[test]
    fn refuse_de_depouiller_sans_bulletin() {
        let outcome = MajorityJudgment.tally(&[], &VotingParams::single_winner());
        assert!(matches!(outcome, Err(VotingError::NoBallots)));
    }

    #[test]
    fn respecte_le_nombre_de_sieges_demande() {
        let ballots = vec![ballot(&[("a", 5), ("b", 4), ("c", 3)])];
        let outcome = MajorityJudgment
            .tally(
                &ballots,
                &VotingParams {
                    seats: 2,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(outcome.winners.len(), 2);
        assert_eq!(outcome.winners[0], "a");
    }
}
