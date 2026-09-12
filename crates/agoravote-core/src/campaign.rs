//! Entité `Campaign` — cf. cahier des charges §5 :
//! « Conteneur d'un processus : titre, statut, dates, visibilité, phases ».
//!
//! Une campagne est le conteneur racine d'un sondage, d'une
//! consultation ou d'un scrutin (cf. §3.2 « Distinction des usages »).
//! Elle référence un [`crate::Form`] (ce qu'on collecte) et, si elle
//! comporte un scrutin, une méthode de vote identifiée par son id de
//! module (cf. §6, §7).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Id;

/// Cycle de vie d'une campagne.
///
/// Les transitions autorisées sont volontairement strictes et
/// linéaires pour le MVP (§15) : `Draft -> Published -> Closed`,
/// avec un aller simple. Une campagne fermée est immuable — c'est ce
/// qui permet de respecter la règle §5.1 « les paramètres du scrutin
/// doivent être conservés avec une version immuable au moment de la
/// clôture ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignStatus {
    /// En cours de configuration, non visible des participants.
    Draft,
    /// Ouverte à la participation.
    Published,
    /// Clôturée : les bulletins ne sont plus acceptés, le résultat
    /// peut être calculé (ou a déjà été calculé) et devient figé.
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: Id,
    pub organization_id: Id,

    pub title: String,
    pub status: CampaignStatus,

    /// Identifiant du formulaire associé (structure de collecte).
    /// `None` tant que le formulaire n'a pas été créé.
    pub form_id: Option<Id>,

    /// Identifiant du module de méthode de vote choisi pour cette
    /// campagne (ex: "voting.majority"), s'il s'agit d'un scrutin et
    /// pas d'un simple sondage sans décision (cf. §3.2).
    ///
    /// On stocke ici l'id *et* la version au moment de la publication
    /// (`voting_method_version`), jamais "la dernière version
    /// disponible" : c'est la garantie de traçabilité exigée en §5.1
    /// (« un résultat doit référencer la version exacte de la méthode
    /// de vote utilisée »).
    pub voting_method_id: Option<String>,
    pub voting_method_version: Option<String>,

    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
}

impl Campaign {
    pub fn new_draft(organization_id: Id, title: impl Into<String>) -> Self {
        Self {
            id: Id::new_v4(),
            organization_id,
            title: title.into(),
            status: CampaignStatus::Draft,
            form_id: None,
            voting_method_id: None,
            voting_method_version: None,
            created_at: Utc::now(),
            published_at: None,
            closed_at: None,
        }
    }

    /// Publie la campagne : elle devient accessible aux participants.
    ///
    /// Refuse la publication si aucun formulaire n'est encore associé —
    /// c'est un exemple concret du critère d'acceptation §18 « les
    /// erreurs de configuration critiques sont détectées avant
    /// publication » : on ne laisse pas partir une campagne vide.
    pub fn publish(&mut self) -> Result<(), CampaignError> {
        if self.form_id.is_none() {
            return Err(CampaignError::MissingForm);
        }
        if self.status != CampaignStatus::Draft {
            return Err(CampaignError::InvalidTransition {
                from: self.status,
                to: CampaignStatus::Published,
            });
        }
        self.status = CampaignStatus::Published;
        self.published_at = Some(Utc::now());
        Ok(())
    }

    /// Clôture la campagne : plus aucun bulletin ne peut être soumis
    /// après cet instant. Cf. §5.1 : à partir d'ici, les paramètres du
    /// scrutin (méthode + version) sont considérés figés.
    pub fn close(&mut self) -> Result<(), CampaignError> {
        if self.status != CampaignStatus::Published {
            return Err(CampaignError::InvalidTransition {
                from: self.status,
                to: CampaignStatus::Closed,
            });
        }
        self.status = CampaignStatus::Closed;
        self.closed_at = Some(Utc::now());
        Ok(())
    }

    /// Vrai si la campagne accepte actuellement des bulletins.
    pub fn accepts_ballots(&self) -> bool {
        self.status == CampaignStatus::Published
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CampaignError {
    #[error("impossible de publier une campagne sans formulaire associé")]
    MissingForm,
    #[error("transition de statut invalide : {from:?} -> {to:?}")]
    InvalidTransition {
        from: CampaignStatus,
        to: CampaignStatus,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuse_de_publier_sans_formulaire() {
        let mut c = Campaign::new_draft(Id::new_v4(), "Budget participatif 2026");
        assert!(matches!(c.publish(), Err(CampaignError::MissingForm)));
    }

    #[test]
    fn cycle_de_vie_nominal() {
        let mut c = Campaign::new_draft(Id::new_v4(), "Budget participatif 2026");
        c.form_id = Some(Id::new_v4());
        c.publish().unwrap();
        assert!(c.accepts_ballots());
        c.close().unwrap();
        assert!(!c.accepts_ballots());
    }

    #[test]
    fn refuse_de_cloturer_un_brouillon() {
        let mut c = Campaign::new_draft(Id::new_v4(), "Test");
        assert!(matches!(
            c.close(),
            Err(CampaignError::InvalidTransition { .. })
        ));
    }
}
