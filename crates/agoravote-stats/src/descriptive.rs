//! Statistiques descriptives — cf. cahier des charges §8.3 :
//! fréquence, proportion, moyenne, médiane, mode, variance,
//! écart-type, quartile.
//!
//! Toutes les fonctions renvoient `Option`/`Vec` vide sur une entrée
//! vide plutôt que de paniquer : un jeu de réponses vide (question
//! non répondue par personne) est un cas normal en cours de campagne,
//! pas une erreur de programmation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Table de fréquence d'une variable catégorielle (§8.3 "Fréquence").
///
/// Renvoie le nombre d'occurrences de chaque valeur distincte, triée
/// par fréquence décroissante puis par ordre alphabétique de la
/// valeur (pour un rendu déterministe, ex. dans un tableau ou un
/// export CSV — §14).
pub fn frequency_table(values: &[String]) -> Vec<(String, u64)> {
    let mut counts: HashMap<&str, u64> = HashMap::new();
    for v in values {
        *counts.entry(v.as_str()).or_insert(0) += 1;
    }
    let mut table: Vec<(String, u64)> = counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    table.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    table
}

/// Proportion (§8.3) de chaque valeur catégorielle dans l'ensemble,
/// exprimée en fraction de 0.0 à 1.0 (à multiplier par 100 pour un
/// pourcentage à l'affichage — cette fonction reste volontairement
/// unitless, la mise en forme est un problème de couche de
/// présentation, cf. §9).
pub fn proportions(values: &[String]) -> HashMap<String, f64> {
    let total = values.len() as f64;
    if total == 0.0 {
        return HashMap::new();
    }
    frequency_table(values)
        .into_iter()
        .map(|(value, count)| (value, count as f64 / total))
        .collect()
}

/// Moyenne arithmétique (§8.3 "Moyenne").
pub fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

/// Médiane (§8.3) : valeur séparant les observations en deux moitiés.
/// Pour un nombre pair d'observations, on retient la moyenne des deux
/// valeurs centrales (convention usuelle).
///
/// Preuve de sûreté des indexations ci-dessous : la fonction retourne
/// tôt si `values` est vide, donc `sorted.len() >= 1` pour le reste du
/// corps ; `mid = sorted.len() / 2` est donc toujours un index valide,
/// et `mid - 1` aussi puisqu'il n'est atteint que dans la branche
/// `len() % 2 == 0`, qui implique `len() >= 2`. On documente cette
/// preuve plutôt que d'utiliser des `.get(..)` défensifs qui
/// masqueraient un cas mathématiquement impossible derrière un
/// `Option` supplémentaire à gérer par l'appelant.
#[allow(clippy::indexing_slicing)]
pub fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    } else {
        Some(sorted[mid])
    }
}

/// Mode (§8.3) : la ou les valeurs les plus fréquentes. Renvoyé comme
/// `Vec` car un jeu de données peut être multimodal (plusieurs
/// valeurs ex-æquo au sommet).
///
/// Les valeurs `f64` sont comparées par égalité stricte : pour des
/// données continues (peu de doublons exacts attendus), le mode a peu
/// de sens statistique — cette fonction est surtout destinée aux
/// données ordinales/discrètes (échelles, notes).
pub fn mode(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }
    // On regroupe par représentation en bits pour permettre l'usage de
    // f64 comme clé de HashMap sans dépendre d'une crate tierce
    // (f64 n'implémente pas Eq/Hash à cause de NaN).
    let mut counts: HashMap<u64, u64> = HashMap::new();
    for &v in values {
        *counts.entry(v.to_bits()).or_insert(0) += 1;
    }
    // Sûr : `values` est non vide (vérifié ci-dessus) donc `counts`
    // contient au moins une entrée, donc `.max()` sur ses valeurs ne
    // peut pas être `None`. `.expect(...)` documente cette garantie au
    // lieu d'un `.unwrap()` silencieux.
    let max_count = *counts
        .values()
        .max()
        .expect("`counts` est non vide car `values` est non vide (vérifié plus haut)");
    let mut modes: Vec<f64> = counts
        .into_iter()
        .filter(|(_, c)| *c == max_count)
        .map(|(bits, _)| f64::from_bits(bits))
        .collect();
    modes.sort_by(|a, b| a.total_cmp(b));
    modes
}

/// Variance (§8.3) : mesure de dispersion quadratique.
///
/// On calcule ici la variance **d'échantillon** (division par n-1,
/// correction de Bessel), plus adaptée quand les répondants sont
/// considérés comme un échantillon d'une population plus large — ce
/// qui est le cas par défaut pour un sondage/consultation (§3.2). Une
/// variante "population" (division par n) pourra être ajoutée si un
/// besoin de recensement exhaustif se présente ; elle n'est pas
/// nécessaire pour le MVP.
pub fn variance(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let m = mean(values)?;
    let sum_sq_diff: f64 = values.iter().map(|v| (v - m).powi(2)).sum();
    Some(sum_sq_diff / (values.len() as f64 - 1.0))
}

/// Écart-type (§8.3) : racine carrée de la variance.
pub fn std_dev(values: &[f64]) -> Option<f64> {
    variance(values).map(f64::sqrt)
}

/// Quartiles (§8.3) : Q1 (25e centile), Q2 (médiane), Q3 (75e centile).
///
/// Méthode utilisée : "exclusive median method" — Q1 et Q3 sont la
/// médiane de la moitié inférieure/supérieure des données, la valeur
/// centrale étant exclue des deux moitiés si le nombre d'observations
/// est impair. C'est une méthode simple et largement enseignée ; elle
/// diffère légèrement d'autres conventions (ex: interpolation
/// linéaire utilisée par certains tableurs) — ce choix est documenté
/// ici précisément pour éviter toute ambiguïté lors d'un audit de
/// résultat (§13, §18).
///
/// Preuve de sûreté du découpage `&sorted[..n/2]` / `&sorted[n/2..]` :
/// `n/2` est toujours compris entre `0` et `n` inclus pour `n >= 0`, ce
/// qui est la seule condition requise pour qu'un slicing Rust soit
/// valide — aucune valeur de `n` ne peut donc faire paniquer ces lignes.
#[allow(clippy::indexing_slicing)]
pub fn quartiles(values: &[f64]) -> Option<(f64, f64, f64)> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let n = sorted.len();
    let q2 = median(&sorted)?;

    let (lower, upper) = if n.is_multiple_of(2) {
        (&sorted[..n / 2], &sorted[n / 2..])
    } else {
        (&sorted[..n / 2], &sorted[n / 2 + 1..])
    };

    let q1 = median(lower).unwrap_or(q2);
    let q3 = median(upper).unwrap_or(q2);
    Some((q1, q2, q3))
}

/// Résumé statistique complet d'une série numérique, regroupant les
/// indicateurs ci-dessus. Pratique pour l'écran "12. Analyse" (§10.1) :
/// un seul appel plutôt qu'une fonction par indicateur.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptiveSummary {
    pub count: usize,
    pub mean: Option<f64>,
    pub median: Option<f64>,
    pub mode: Vec<f64>,
    pub variance: Option<f64>,
    pub std_dev: Option<f64>,
    pub q1: Option<f64>,
    pub q3: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl DescriptiveSummary {
    pub fn compute(values: &[f64]) -> Self {
        let q = quartiles(values);
        Self {
            count: values.len(),
            mean: mean(values),
            median: median(values),
            mode: mode(values),
            variance: variance(values),
            std_dev: std_dev(values),
            q1: q.map(|(q1, _, _)| q1),
            q3: q.map(|(_, _, q3)| q3),
            min: values
                .iter()
                .cloned()
                .fold(None, |acc, v| Some(acc.map_or(v, |m: f64| m.min(v)))),
            max: values
                .iter()
                .cloned()
                .fold(None, |acc, v| Some(acc.map_or(v, |m: f64| m.max(v)))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moyenne_mediane_simples() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(mean(&values), Some(3.0));
        assert_eq!(median(&values), Some(3.0));
    }

    #[test]
    fn mediane_nombre_pair_dobservations() {
        let values = vec![1.0, 2.0, 3.0, 4.0];
        assert_eq!(median(&values), Some(2.5));
    }

    #[test]
    fn mode_multimodal() {
        let values = vec![1.0, 2.0, 2.0, 3.0, 3.0];
        let mut m = mode(&values);
        m.sort_by(|a, b| a.total_cmp(b));
        assert_eq!(m, vec![2.0, 3.0]);
    }

    #[test]
    fn variance_et_ecart_type_connus() {
        // Jeu de données classique : variance d'échantillon = 4.5714...
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let v = variance(&values).unwrap();
        assert!((v - 4.5714285714).abs() < 1e-6);
        let sd = std_dev(&values).unwrap();
        assert!((sd - v.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn quartiles_sur_neuf_valeurs() {
        let values: Vec<f64> = (1..=9).map(|x| x as f64).collect(); // 1..9
        let (q1, q2, q3) = quartiles(&values).unwrap();
        // Méthode "exclusive median" (cf. doc de `quartiles`) : la
        // valeur centrale (5) est exclue des deux moitiés [1..4] et
        // [6..9], dont les médianes sont 2.5 et 7.5.
        assert_eq!(q2, 5.0);
        assert_eq!(q1, 2.5);
        assert_eq!(q3, 7.5);
    }

    #[test]
    fn table_de_frequence_triee() {
        let values = vec!["A", "B", "A", "C", "A", "B"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let table = frequency_table(&values);
        assert_eq!(table[0], ("A".to_string(), 3));
    }

    #[test]
    fn liste_vide_ne_panique_pas() {
        assert_eq!(mean(&[]), None);
        assert_eq!(median(&[]), None);
        assert_eq!(variance(&[]), None);
        assert_eq!(quartiles(&[]), None);
        assert!(mode(&[]).is_empty());
    }
}
