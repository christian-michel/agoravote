//! Entité `User` — cf. cahier des charges §5 :
//! « Identité applicative et profil public/privé du participant :
//! identifiant, profil, rôles, préférences de langue, historique de
//! participation ».

use serde::{Deserialize, Serialize};

use crate::Id;

/// Un utilisateur de la plateforme : administrateur, gestionnaire de
/// campagne ou simple participant. Le même type sert aux trois cas
/// (un participant peut devenir gestionnaire), le rôle effectif étant
/// porté par [`Role`] et évalué par organisation.
///
/// Ce type ne gère PAS l'authentification (mots de passe, sessions,
/// MFA — cf. §13) : c'est un profil applicatif, pas un compte
/// d'identité. La couche d'authentification est un sujet à part,
/// volontairement hors périmètre de ce crate de modèle pur.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Id,

    pub organization_id: Id,

    /// Nom affiché. Peut être pseudonymisé pour les scrutins qui
    /// exigent la séparation identité/bulletin (§13, "Secret du vote").
    pub display_name: String,

    /// Langue préférée de l'utilisateur (code BCP 47). Utilisée pour
    /// choisir la traduction des campagnes multilingues (§1.1).
    pub preferred_language: Option<String>,

    /// Rôles détenus par l'utilisateur au sein de son organisation.
    /// Un utilisateur peut cumuler plusieurs rôles (ex: `Organizer`
    /// sur une campagne et `Voter` sur une autre) — cf. §13 « RBAC /
    /// permissions fines ».
    pub roles: Vec<Role>,
}

/// Rôles applicatifs de haut niveau (RBAC minimal pour le MVP, §15).
///
/// Ce catalogue est volontairement restreint : le cahier des charges
/// (§13) demande des « permissions fines », qui seront introduites
/// plus tard sous forme de permissions granulaires par ressource.
/// Ces rôles composites suffisent au MVP et servent de première brique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Administrateur de l'organisation : gère utilisateurs, modules,
    /// paramètres globaux (écran "16. Modules", écran "Paramètres").
    Admin,
    /// Peut créer et administrer des campagnes (écrans 01 à 08).
    Organizer,
    /// Peut participer aux campagnes qui lui sont ouvertes (écran 09).
    Voter,
    /// Accès en lecture seule aux résultats et à l'audit (écrans 11,
    /// 12, 15) — utile pour un rôle "observateur"/scrutateur.
    Auditor,
}

impl User {
    pub fn new(organization_id: Id, display_name: impl Into<String>) -> Self {
        Self {
            id: Id::new_v4(),
            organization_id,
            display_name: display_name.into(),
            preferred_language: None,
            roles: vec![Role::Voter],
        }
    }

    /// Vrai si l'utilisateur détient le rôle donné.
    pub fn has_role(&self, role: Role) -> bool {
        self.roles.contains(&role)
    }
}
