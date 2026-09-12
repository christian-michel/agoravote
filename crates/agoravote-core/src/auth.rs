//! Entités `Account` et `Session` — cf. cahier des charges §13 :
//! « Authentification : Sessions sécurisées, MFA optionnel selon
//! contexte » et « Autorisation : RBAC / permissions fines ».
//!
//! Ces types sont volontairement de purs conteneurs de données (comme
//! le reste de ce crate, cf. `lib.rs`) : la logique de hachage de mot
//! de passe, de génération de jeton et de vérification vit dans le
//! crate séparé `agoravote-auth` (même principe que
//! `VotingMethod` défini ici mais implémenté dans `agoravote-voting`).
//! `agoravote-core` ne sait donc pas CE QU'EST un mot de passe en clair
//! ni COMMENT un jeton est produit — seulement à quoi ressemblent un
//! compte et une session une fois qu'ils existent.
//!
//! ## Pourquoi `Account` est séparé de `User`
//!
//! `User` (cf. `user.rs`) est un profil applicatif : nom affiché,
//! rôles, préférences. `Account` porte les identifiants de connexion
//! (email, empreinte du mot de passe). Cette séparation permet, par
//! exemple, un jour un utilisateur avec plusieurs moyens de connexion
//! (mot de passe ET Ğ1, cf. l'addendum v0.3 du cahier des charges)
//! sans dupliquer son profil.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Id;

/// Identifiants de connexion d'un utilisateur.
///
/// `password_hash` est une chaîne au format PHC (ex:
/// `$argon2id$v=19$m=19456,t=2,p=1$...`), jamais le mot de passe en
/// clair — cette chaîne encode déjà l'algorithme, ses paramètres et le
/// sel, donc `agoravote-store` n'a besoin d'aucune colonne
/// supplémentaire pour la vérifier plus tard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: Id,
    pub user_id: Id,

    /// Normalisé en minuscules avant stockage (cf. `agoravote-auth`) :
    /// deux comptes ne doivent pas pouvoir exister avec la même
    /// adresse à la casse près.
    pub email: String,

    pub password_hash: String,

    pub created_at: DateTime<Utc>,
}

impl Account {
    pub fn new(user_id: Id, email: impl Into<String>, password_hash: impl Into<String>) -> Self {
        Self {
            id: Id::new_v4(),
            user_id,
            email: email.into(),
            password_hash: password_hash.into(),
            created_at: Utc::now(),
        }
    }
}

/// Session active d'un utilisateur authentifié.
///
/// ## Pourquoi un jeton opaque plutôt qu'un JWT
///
/// Un jeton auto-porteur (JWT) encoderait l'identité et les rôles
/// directement dans le jeton, signé mais pas révocable avant son
/// expiration naturelle : impossible de forcer une déconnexion
/// immédiate (compte compromis, rôle changé) sans liste de révocation
/// — qui revient de toute façon à réintroduire un état côté serveur,
/// donc autant garder un jeton opaque et un état de session simple
/// dès le départ (§13, "Sessions sécurisées"). Le prix : une requête
/// de vérification de session lit le store à chaque appel — un coût
/// négligeable au regard de la simplicité gagnée.
///
/// ## Pourquoi le jeton stocké n'est jamais le jeton en clair
///
/// Ce type ne contient PAS le jeton en clair : il contient
/// `token_hash`, l'empreinte SHA-256 du jeton (cf. `agoravote-auth`).
/// Si la base de données venait à être exposée (fuite, sauvegarde mal
/// protégée), un attaquant n'obtiendrait aucune session utilisable
/// directement — exactement le même raisonnement que pour un mot de
/// passe, appliqué ici à un secret différent. Le jeton en clair n'est
/// renvoyé qu'une seule fois, au moment de la connexion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub token_hash: String,
    pub user_id: Id,
    pub organization_id: Id,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl Session {
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }
}

/// Lien entre un [`crate::User`] et une clé publique Ğ1v2 — second
/// moyen de connexion optionnel, en plus (jamais à la place) du
/// couple email/mot de passe porté par [`Account`], cf. l'addendum
/// v0.3 du cahier des charges (« Identité décentralisée ») et la note
/// ci-dessus sur la séparation `User`/`Account`.
///
/// Ne contient que la clé **publique** (hex, 32 octets sr25519) : la
/// clé privée et la phrase de 12 mots d'un participant ne transitent
/// jamais vers AgoraVote — cf. `agoravote_g1::signature`, dont la
/// vérification de preuve de possession est le seul mécanisme par
/// lequel ce lien peut être créé (jamais une simple déclaration de
/// clé non prouvée).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct G1Link {
    pub id: Id,
    pub user_id: Id,

    /// Encodage hexadécimal (sans préfixe `0x`) des 32 octets de la
    /// clé publique sr25519 — même convention que
    /// `agoravote_g1::signature::decode_hex`. Unique : un compte Ğ1 ne
    /// peut être lié qu'à un seul utilisateur AgoraVote (même logique
    /// que la contrainte d'unicité d'email sur `Account`).
    pub public_key_hex: String,

    pub linked_at: DateTime<Utc>,
}

impl G1Link {
    pub fn new(user_id: Id, public_key_hex: impl Into<String>) -> Self {
        Self {
            id: Id::new_v4(),
            user_id,
            public_key_hex: public_key_hex.into(),
            linked_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn detecte_une_session_expiree() {
        let now = Utc::now();
        let session = Session {
            token_hash: "peu importe ici".to_string(),
            user_id: Id::new_v4(),
            organization_id: Id::new_v4(),
            created_at: now - Duration::hours(2),
            expires_at: now - Duration::hours(1),
        };
        assert!(session.is_expired(now));
    }

    #[test]
    fn une_session_recente_nest_pas_expiree() {
        let now = Utc::now();
        let session = Session {
            token_hash: "peu importe ici".to_string(),
            user_id: Id::new_v4(),
            organization_id: Id::new_v4(),
            created_at: now,
            expires_at: now + Duration::hours(1),
        };
        assert!(!session.is_expired(now));
    }
}
