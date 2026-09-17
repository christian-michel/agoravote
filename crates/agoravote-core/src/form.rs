//! Entités `Form` et `Question` — cf. cahier des charges §5 et §6.2
//! (« Familles de modules envisagées » → types de question).
//!
//! Un `Form` est la structure de collecte d'une campagne (§5) ; il
//! contient une ou plusieurs `Question`. Le périmètre MVP (§15) ne
//! couvre que les types de question suivants : choix unique, choix
//! multiple, texte, nombre, échelle, classement, jugement majoritaire.
//! Les autres familles listées en §6.2 (matrice, date...) seront
//! ajoutées comme variantes supplémentaires de [`QuestionType`] sans
//! changer le reste du modèle.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Id;

/// Type d'une question : décrit CE QUE le participant peut saisir.
///
/// Important (§7 « Méthodes de vote et règles ») : le type de question
/// décrit la *saisie*, pas la méthode de décision. Une question
/// `SingleChoice` peut être dépouillée par une méthode "majorité
/// simple" comme par une méthode "quorum + majorité qualifiée" : le
/// type de question n'impose pas la méthode de vote, il en délimite
/// seulement les entrées compatibles (cf. `ModuleManifest::inputs`
/// dans [`crate::voting_method`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QuestionType {
    /// Un seul choix parmi une liste d'options.
    SingleChoice { options: Vec<QuestionOption> },
    /// Plusieurs choix possibles parmi une liste d'options (vote
    /// d'approbation quand utilisé comme scrutin — cf. §8.2).
    MultipleChoice {
        options: Vec<QuestionOption>,
        /// Nombre maximum d'options sélectionnables, si limité.
        max_selections: Option<u32>,
    },
    /// Réponse libre.
    Text { max_length: Option<u32> },
    /// Réponse numérique (utilisée par exemple pour le vote par score,
    /// §8.2, en combinaison avec une méthode `voting.score`).
    Number { min: Option<f64>, max: Option<f64> },
    /// Échelle bornée (ex: 1 à 5, 0 à 10).
    Scale { min: i32, max: i32 },
    /// Classement d'options par ordre de préférence (utilisé par les
    /// méthodes de vote préférentiel : Condorcet, STV — §8.2).
    Ranking { options: Vec<QuestionOption> },
    /// Jugement majoritaire (§6.2, §15.1) : chaque participant
    /// attribue une MENTION (échelle qualitative fixe, 1 à 6 : de
    /// "Très défavorable" à "Très favorable", plus "Ne sait pas") à
    /// CHAQUE option — pas une seule sélection comme `SingleChoice`,
    /// ni une note libre comme pour `voting.score`. Type dédié plutôt
    /// que réutiliser `SingleChoice`/`Number` : l'échelle de mentions
    /// est fixe et partagée par toutes les options d'une même
    /// question, ce qu'aucun des deux autres types ne peut exprimer.
    /// Seule `voting.majority_judgment` sait dépouiller ce type (cf.
    /// `agoravote-voting`).
    MajorityJudgment { options: Vec<QuestionOption> },
}

/// Une option proposée pour une question à choix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    /// Identifiant stable de l'option (référencé dans les bulletins
    /// et dans les résultats — ne doit jamais être un simple index de
    /// tableau, pour rester stable si l'ordre change).
    pub id: String,
    /// Libellés traduits, par code langue (§1.1 multilinguisme natif).
    pub labels: HashMap<String, String>,
}

impl QuestionOption {
    pub fn new(id: impl Into<String>, label_fr: impl Into<String>) -> Self {
        let mut labels = HashMap::new();
        labels.insert("fr".to_string(), label_fr.into());
        Self {
            id: id.into(),
            labels,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: Id,
    pub form_id: Id,

    /// Libellés traduits de l'intitulé de la question.
    pub prompt: HashMap<String, String>,

    pub question_type: QuestionType,

    /// Une question obligatoire bloque la progression du formulaire
    /// tant qu'elle n'a pas de réponse (logique conditionnelle simple,
    /// écran "05. Logique conditionnelle", §10.1).
    pub required: bool,

    /// Position d'affichage dans le formulaire.
    pub position: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Form {
    pub id: Id,
    pub campaign_id: Id,
    pub questions: Vec<Question>,
}

impl Form {
    pub fn new(campaign_id: Id) -> Self {
        Self {
            id: Id::new_v4(),
            campaign_id,
            questions: Vec::new(),
        }
    }

    /// Ajoute une question en fin de formulaire et lui assigne
    /// automatiquement la position suivante.
    pub fn add_question(
        &mut self,
        prompt_fr: impl Into<String>,
        question_type: QuestionType,
        required: bool,
    ) -> Id {
        let mut prompt = HashMap::new();
        prompt.insert("fr".to_string(), prompt_fr.into());
        let position = self.questions.len() as u32;
        let question = Question {
            id: Id::new_v4(),
            form_id: self.id,
            prompt,
            question_type,
            required,
            position,
        };
        let id = question.id;
        self.questions.push(question);
        id
    }
}
