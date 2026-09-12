//! Analyse croisée — cf. cahier des charges §8.4 :
//! « Croisement / tableau croisé (cross-tabulation) : présentation
//! conjointe de deux ou plusieurs variables catégorielles. »
//!
//! §8 rappelle explicitement le garde-fou méthodologique : *« Une
//! interface peut calculer et afficher une proportion sans conclure
//! que cette proportion représente une opinion de toute une
//! population »*. Ce module se contente donc de produire des
//! comptages ; toute mention de significativité, de marge d'erreur ou
//! d'interprétation revient à la couche de présentation (§9), jamais
//! à ce moteur.

use std::collections::HashMap;

/// Cellule d'un tableau croisé : une paire (valeur de la ligne, valeur
/// de la colonne) et le nombre d'observations correspondant.
pub type CrossTabCell = ((String, String), u64);

/// Calcule un tableau croisé entre deux variables catégorielles
/// appariées (`rows[i]` et `cols[i]` décrivent la même observation
/// i, par exemple : réponse à la question A et réponse à la question
/// B pour un même participant).
///
/// # Panics
/// En mode debug, panique si `rows.len() != cols.len()` : les deux
/// séries doivent être appariées observation par observation. C'est
/// une erreur d'appel (bug côté appelant), pas un cas métier normal —
/// contrairement aux séries vides, qui sont gérées sans panique.
pub fn cross_tabulation(rows: &[String], cols: &[String]) -> Vec<CrossTabCell> {
    debug_assert_eq!(
        rows.len(),
        cols.len(),
        "cross_tabulation: les deux séries doivent avoir la même longueur (observations appariées)"
    );

    let mut counts: HashMap<(String, String), u64> = HashMap::new();
    for (r, c) in rows.iter().zip(cols.iter()) {
        *counts.entry((r.clone(), c.clone())).or_insert(0) += 1;
    }

    let mut cells: Vec<CrossTabCell> = counts.into_iter().collect();
    cells.sort_by(|a, b| a.0.cmp(&b.0));
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn croise_deux_variables_categorielles() {
        let age_group = vec!["18-25", "18-25", "26-40", "26-40"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let choice = vec!["A", "B", "A", "A"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();

        let table = cross_tabulation(&age_group, &choice);

        assert!(table.contains(&(("18-25".to_string(), "A".to_string()), 1)));
        assert!(table.contains(&(("18-25".to_string(), "B".to_string()), 1)));
        assert!(table.contains(&(("26-40".to_string(), "A".to_string()), 2)));
    }

    #[test]
    fn series_vides_ne_paniquent_pas() {
        let table = cross_tabulation(&[], &[]);
        assert!(table.is_empty());
    }
}
