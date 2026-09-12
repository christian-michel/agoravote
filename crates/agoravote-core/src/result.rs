//! Entité `ResultSet` — cf. cahier des charges §5 :
//! « Résultat calculé : compteurs, proportions, gagnants, métadonnées »
//! et §5.1 « un résultat doit référencer la version exacte de la
//! méthode de vote utilisée ».
//!
//! Alors que [`crate::voting_method::TallyOutcome`] est la sortie
//! immédiate d'un appel à `VotingMethod::tally`, `ResultSet` est
//! l'enveloppe **persistée** qui l'accompagne de tout ce qu'il faut
//! pour l'auditer et l'afficher sans ambiguïté plus tard : quelle
//! campagne, quelle question, quelle méthode et quelle version
//! exacte, à quel instant. C'est cette enveloppe que les
//! visualisations référencent (§5.1 : « une visualisation doit
//! référencer le jeu de résultats qu'elle présente »), jamais les
//! bulletins bruts directement.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::voting_method::TallyOutcome;
use crate::Id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSet {
    pub id: Id,
    pub campaign_id: Id,
    pub question_id: Id,

    /// Id et version exacte du module de méthode de vote utilisé.
    /// Toujours renseignés ensemble : on ne référence jamais "la
    /// dernière version" d'un module (§5.1).
    pub voting_method_id: String,
    pub voting_method_version: String,

    pub outcome: TallyOutcome,

    /// Instant auquel ce résultat a été calculé. Un `ResultSet` n'est
    /// jamais modifié après création : un recalcul (ex: changement de
    /// méthode) produit un nouveau `ResultSet`, jamais une mutation en
    /// place (§4 : "un changement de méthode de vote doit produire un
    /// nouveau résultat calculé").
    pub computed_at: DateTime<Utc>,
}

impl ResultSet {
    pub fn new(
        campaign_id: Id,
        question_id: Id,
        voting_method_id: impl Into<String>,
        voting_method_version: impl Into<String>,
        outcome: TallyOutcome,
    ) -> Self {
        Self {
            id: Id::new_v4(),
            campaign_id,
            question_id,
            voting_method_id: voting_method_id.into(),
            voting_method_version: voting_method_version.into(),
            outcome,
            computed_at: Utc::now(),
        }
    }
}
