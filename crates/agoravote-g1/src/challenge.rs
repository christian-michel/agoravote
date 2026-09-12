//! Défi (nonce) à signer — première moitié du protocole
//! d'authentification par preuve de possession de clé.
//!
//! Principe (cf. le tableau de l'échange précédent avec l'utilisateur) :
//! AgoraVote génère un défi aléatoire et l'envoie au client ; le
//! client le signe **localement**, dans son portefeuille (Cesium²,
//! Ğecko, extension navigateur...), avec la clé privée dérivée de sa
//! phrase de 12 mots — laquelle ne quitte JAMAIS l'appareil de
//! l'utilisateur ; AgoraVote ne reçoit que le résultat signé
//! ([`crate::signature::verify_signature`]).
//!
//! Ce module ne fait aucune I/O et ne dépend d'aucune bibliothèque
//! réseau — contrairement au reste du crate, sa logique est simple
//! d'implémenter correctement même sans pouvoir la tester ici.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Durée de validité d'un défi avant qu'il ne doive être régénéré —
/// une fenêtre courte limite le temps disponible pour un rejeu si un
/// défi signé venait à être intercepté (peu probable puisqu'il
/// transite en HTTPS, mais une défense en profondeur peu coûteuse).
pub const CHALLENGE_VALIDITY: Duration = Duration::minutes(5);

/// Un défi à signer, associé à l'instant après lequel il n'est plus
/// valide.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    /// Représentation textuelle du défi, telle qu'elle doit être
    /// signée par le portefeuille de l'utilisateur (encodage UTF-8
    /// des octets signés — convention à confirmer contre le
    /// comportement réel des portefeuilles Ğ1 lors de la vérification
    /// en conditions réelles, cf. avertissement de `lib.rs`).
    pub message: String,
    pub expires_at: DateTime<Utc>,
}

impl Challenge {
    /// Génère un nouveau défi, aléatoire et non réutilisable (deux
    /// appels ne produisent jamais le même message — cf. test
    /// ci-dessous), valide [`CHALLENGE_VALIDITY`] à partir de
    /// maintenant.
    ///
    /// Le message inclut un préfixe explicite ("AgoraVote souhaite
    /// vérifier...") : de nombreux portefeuilles affichent le texte
    /// signé à l'utilisateur avant signature — un message clair évite
    /// qu'il signe un texte qu'il ne comprend pas.
    pub fn generate() -> Self {
        let nonce = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        Self {
            message: format!(
                "AgoraVote souhaite vérifier que vous contrôlez ce compte Ğ1. \
                 Ceci n'est PAS une transaction. Code : {nonce}"
            ),
            expires_at: Utc::now() + CHALLENGE_VALIDITY,
        }
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deux_defis_generes_sont_differents() {
        let a = Challenge::generate();
        let b = Challenge::generate();
        assert_ne!(a.message, b.message);
    }

    #[test]
    fn un_defi_frais_nest_pas_expire() {
        let challenge = Challenge::generate();
        assert!(!challenge.is_expired(Utc::now()));
    }

    #[test]
    fn un_defi_expire_apres_sa_duree_de_validite() {
        let challenge = Challenge::generate();
        let bien_plus_tard = Utc::now() + CHALLENGE_VALIDITY + Duration::seconds(1);
        assert!(challenge.is_expired(bien_plus_tard));
    }
}
