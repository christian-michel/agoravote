//! Entité `Ballot` — cf. cahier des charges §5 :
//! « Réponse ou bulletin : données, état, horodatage, preuve selon
//! mode » et §5.1 « les réponses brutes doivent être distinguables
//! des résultats calculés ».
//!
//! Un `Ballot` est une donnée BRUTE : ce que le participant a
//! effectivement saisi pour une question donnée, avant toute règle de
//! validité (§7, "Règle" / §8.1) et avant tout calcul (§7, "Méthode").
//! Un `Ballot` ne dit pas "ce vote compte" ou "ce vote est un vote
//! blanc" — ce jugement est produit par le moteur de règles au moment
//! du dépouillement, pas stocké sur le bulletin lui-même. Cela permet
//! de rejouer un dépouillement avec des règles différentes sans avoir
//! à modifier les bulletins déjà enregistrés.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Id;

/// Un bulletin brut soumis pour une question précise.
///
/// Les différents champs de données (`selections`, `scores`,
/// `numeric_value`, `text_value`) correspondent aux différents
/// [`crate::form::QuestionType`]. Un bulletin ne remplit que le(s)
/// champ(s) pertinent(s) pour le type de question auquel il répond ;
/// les autres restent à `None`/vides. On préfère cette forme "large"
/// (un seul type `Ballot`) à un enum de bulletins par type de
/// question : cela simplifie le stockage et les exports (§14), la
/// cohérence avec le type de question étant vérifiée à la création
/// via [`Ballot::for_single_choice`] et consorts plutôt que par le
/// système de types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ballot {
    pub id: Id,
    pub campaign_id: Id,
    pub question_id: Id,

    /// Identifiant du votant, si la campagne n'exige pas le secret du
    /// vote. `None` quand l'identité et le bulletin sont séparés
    /// (cf. §13 "Secret du vote") : dans ce cas, la preuve de
    /// participation (émargement) est stockée ailleurs, jamais reliée
    /// au contenu du bulletin dans le même enregistrement.
    pub voter_id: Option<Id>,

    /// Options sélectionnées, dans l'ordre de préférence pour un
    /// classement (`Ranking`), sans ordre significatif pour un choix
    /// unique/multiple.
    pub selections: Vec<String>,

    /// Notes attribuées par option, pour le vote par score (§8.2).
    pub scores: Option<HashMap<String, f64>>,

    /// Valeur numérique brute, pour les questions `Number`/`Scale`.
    pub numeric_value: Option<f64>,

    /// Réponse libre, pour les questions `Text`.
    pub text_value: Option<String>,

    pub cast_at: DateTime<Utc>,
}

impl Ballot {
    /// Construit un bulletin pour une question à choix (unique ou
    /// multiple) ou de classement.
    pub fn for_selections(
        campaign_id: Id,
        question_id: Id,
        voter_id: Option<Id>,
        selections: Vec<String>,
    ) -> Self {
        Self {
            id: Id::new_v4(),
            campaign_id,
            question_id,
            voter_id,
            selections,
            scores: None,
            numeric_value: None,
            text_value: None,
            cast_at: Utc::now(),
        }
    }

    /// Construit un bulletin pour une question de vote par score.
    pub fn for_scores(
        campaign_id: Id,
        question_id: Id,
        voter_id: Option<Id>,
        scores: HashMap<String, f64>,
    ) -> Self {
        Self {
            id: Id::new_v4(),
            campaign_id,
            question_id,
            voter_id,
            selections: Vec::new(),
            scores: Some(scores),
            numeric_value: None,
            text_value: None,
            cast_at: Utc::now(),
        }
    }
}
