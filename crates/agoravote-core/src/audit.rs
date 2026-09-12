//! Entité `AuditEvent` — cf. cahier des charges §5 :
//! « Traçabilité : acteur, action, date, contexte » et §13 :
//! « Journal d'audit séparé des données de vote ».
//!
//! Le journal d'audit trace les actions ADMINISTRATIVES (créer,
//! publier, clôturer une campagne, installer un module...), pas les
//! bulletins eux-mêmes : mélanger les deux romprait la séparation
//! identité/bulletin exigée pour les scrutins secrets (§13). C'est
//! pourquoi `AuditEvent` ne référence jamais un `Ballot`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Id,
    pub organization_id: Id,

    /// Utilisateur à l'origine de l'action. `None` pour une action
    /// système automatisée (ex: clôture programmée d'une campagne).
    pub actor_id: Option<Id>,

    /// Description courte et stable de l'action, ex:
    /// "campaign.published", "module.installed". Utiliser un format
    /// de type "ressource.verbe" facilite le filtrage dans l'écran
    /// "15. Audit" (§10.1).
    pub action: String,

    /// Contexte libre, sérialisé en JSON (id de la ressource
    /// concernée, ancienne/nouvelle valeur pour un changement de
    /// paramètre, etc.).
    pub context: serde_json::Value,

    pub occurred_at: DateTime<Utc>,
}

impl AuditEvent {
    pub fn new(
        organization_id: Id,
        actor_id: Option<Id>,
        action: impl Into<String>,
        context: serde_json::Value,
    ) -> Self {
        Self {
            id: Id::new_v4(),
            organization_id,
            actor_id,
            action: action.into(),
            context,
            occurred_at: Utc::now(),
        }
    }
}
