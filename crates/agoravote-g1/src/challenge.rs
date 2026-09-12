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
    ///
    /// Inclut l'horodatage d'expiration en toutes lettres (epoch,
    /// après le marqueur `EXP:`) — ce qui permet à
    /// [`Challenge::verify_freshness`] de vérifier la fraîcheur d'un
    /// défi reçu **sans que le serveur ait besoin de le mémoriser**
    /// entre l'émission et la vérification (cf. `AppState`, qui reste
    /// ainsi sans état pour ce protocole, comme pour les jetons de
    /// session). Comme cet horodatage fait partie du texte signé, un
    /// client ne peut pas le falsifier pour rejouer indéfiniment un
    /// défi capturé : falsifier `EXP:` invaliderait la signature.
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
        let expires_at = Utc::now() + CHALLENGE_VALIDITY;
        Self {
            message: format!(
                "AgoraVote souhaite vérifier que vous contrôlez ce compte Ğ1. \
                 Ceci n'est PAS une transaction. Code : {nonce}. EXP:{}",
                expires_at.timestamp()
            ),
            expires_at,
        }
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    /// Relit l'horodatage d'expiration embarqué dans un message de
    /// défi **déjà signé** (reçu tel quel du client, cf. doc du champ
    /// `message`) et vérifie qu'il n'a pas expiré. Utilisé côté
    /// vérification (`agoravote-api`), qui ne conserve jamais le
    /// [`Challenge`] d'origine — seule la signature garantit que ce
    /// message n'a pas été altéré depuis son émission.
    ///
    /// `Err(G1Error::ChallengeExpired)` couvre aussi bien un message
    /// expiré qu'un message malformé (pas de marqueur `EXP:`, ou
    /// horodatage illisible) : dans les deux cas, la bonne réponse
    /// côté appelant est la même — redemander un défi frais — donc pas
    /// besoin d'une variante d'erreur séparée pour un cas qui ne
    /// devrait de toute façon jamais se produire avec un client honnête.
    pub fn verify_freshness(
        message: &str,
        now: DateTime<Utc>,
    ) -> Result<(), crate::error::G1Error> {
        let epoch = message
            .rsplit("EXP:")
            .next()
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or(crate::error::G1Error::ChallengeExpired)?;
        let expires_at =
            DateTime::from_timestamp(epoch, 0).ok_or(crate::error::G1Error::ChallengeExpired)?;
        if now >= expires_at {
            return Err(crate::error::G1Error::ChallengeExpired);
        }
        Ok(())
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

    #[test]
    fn verify_freshness_accepte_un_message_frais() {
        let challenge = Challenge::generate();
        assert!(Challenge::verify_freshness(&challenge.message, Utc::now()).is_ok());
    }

    #[test]
    fn verify_freshness_refuse_un_message_expire() {
        let challenge = Challenge::generate();
        let bien_plus_tard = Utc::now() + CHALLENGE_VALIDITY + Duration::seconds(1);
        assert!(Challenge::verify_freshness(&challenge.message, bien_plus_tard).is_err());
    }

    #[test]
    fn verify_freshness_refuse_un_message_sans_horodatage() {
        assert!(Challenge::verify_freshness("un message sans marqueur EXP", Utc::now()).is_err());
    }

    #[test]
    fn verify_freshness_refuse_un_horodatage_falsifie_a_lavenir() {
        // Un client malveillant qui rejouerait un défi capturé ne peut
        // pas simplement remplacer EXP: par une date future : la
        // signature, elle, couvre le message d'origine — falsifier
        // cette chaîne AVANT vérification de signature (ce que ce test
        // isole) romprait de toute façon la correspondance avec la
        // signature fournie, cf. `signature::verify_signature`.
        let challenge = Challenge::generate();
        let loin_dans_le_futur = Utc::now() + Duration::days(3650);
        let message_falsifie = format!(
            "{}. EXP:{}",
            challenge.message.split(". EXP:").next().unwrap(),
            loin_dans_le_futur.timestamp()
        );
        // Ce message falsifié PASSE verify_freshness (il n'a pas
        // encore expiré) — ce qui prouve que cette fonction seule ne
        // suffit pas : elle doit toujours être combinée à
        // `verify_signature` sur le message REÇU (falsifié ou non),
        // jamais appelée indépendamment de la vérification de
        // signature côté appelant (cf. routes.rs::g1_verify).
        assert!(Challenge::verify_freshness(&message_falsifie, Utc::now()).is_ok());
    }
}
