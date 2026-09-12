//! Entité `Organization` — cf. cahier des charges §5 :
//! « Périmètre de gouvernance : nom, domaine, paramètres, langues ».

use serde::{Deserialize, Serialize};

use crate::Id;

/// Une organisation est le périmètre de gouvernance racine : c'est
/// elle qui possède les campagnes, les utilisateurs et les modules
/// installés. Une instance auto-hébergée d'AgoraVote peut contenir
/// une ou plusieurs organisations (association, collectif de
/// collectifs, mairie avec plusieurs directions, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Id,

    /// Nom affiché de l'organisation.
    pub name: String,

    /// Liste des langues activées pour cette organisation.
    ///
    /// Conformément au principe directeur « Multilinguisme natif »
    /// (§1.1), la langue n'est pas une option de l'interface : elle
    /// fait partie du modèle de données dès la racine. Chaque
    /// contenu traduisible (question, libellé de campagne...) référence
    /// un sous-ensemble de ces codes langue (BCP 47, ex: "fr", "en").
    pub languages: Vec<String>,

    /// Langue utilisée par défaut quand aucune préférence explicite
    /// n'est disponible (participant anonyme sans langue navigateur
    /// reconnue, par exemple).
    pub default_language: String,
}

impl Organization {
    /// Construit une nouvelle organisation avec un identifiant généré.
    ///
    /// `default_language` doit obligatoirement figurer dans `languages` :
    /// on le vérifie ici plutôt que de laisser un état incohérent se
    /// propager plus loin dans le système (cf. §18, critère d'acceptation
    /// « les erreurs de configuration critiques sont détectées avant
    /// publication » — on l'applique dès la construction de l'objet).
    pub fn new(
        name: impl Into<String>,
        languages: Vec<String>,
        default_language: impl Into<String>,
    ) -> Result<Self, OrganizationError> {
        let default_language = default_language.into();
        if !languages.contains(&default_language) {
            return Err(OrganizationError::DefaultLanguageNotDeclared);
        }
        Ok(Self {
            id: Id::new_v4(),
            name: name.into(),
            languages,
            default_language,
        })
    }
}

/// Erreurs de construction/validation d'une organisation.
#[derive(Debug, thiserror::Error)]
pub enum OrganizationError {
    #[error("la langue par défaut doit figurer dans la liste des langues activées")]
    DefaultLanguageNotDeclared,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuse_une_langue_par_defaut_non_declaree() {
        let result = Organization::new("Ma commune", vec!["fr".into()], "en");
        assert!(matches!(
            result,
            Err(OrganizationError::DefaultLanguageNotDeclared)
        ));
    }

    #[test]
    fn accepte_une_configuration_coherente() {
        let org = Organization::new("Ma commune", vec!["fr".into(), "en".into()], "fr").unwrap();
        assert_eq!(org.default_language, "fr");
    }
}
