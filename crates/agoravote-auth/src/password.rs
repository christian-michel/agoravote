//! Hachage de mot de passe — cf. cahier des charges §13.
//!
//! Algorithme : Argon2id, via le crate `argon2` (implémentation
//! RustCrypto de l'algorithme vainqueur du Password Hashing
//! Competition 2015, recommandé par l'OWASP pour le stockage de mots
//! de passe). Le résultat est une chaîne au format PHC
//! (`$argon2id$v=19$m=...,t=...,p=...$<sel>$<empreinte>`) :
//! l'algorithme, ses paramètres et le sel (aléatoire, différent à
//! chaque hachage même pour un mot de passe identique) sont encodés
//! DANS la chaîne — `agoravote-store` n'a donc besoin de stocker
//! qu'une seule colonne de texte, jamais de sel séparé.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("échec du hachage du mot de passe")]
    HashingFailed,

    #[error("format d'empreinte de mot de passe invalide (corruption en base ?)")]
    InvalidHashFormat,
}

/// Hache un mot de passe en clair. Chaque appel produit un résultat
/// différent (sel aléatoire), même pour un mot de passe identique —
/// c'est voulu : deux comptes avec le même mot de passe ne doivent
/// jamais avoir la même empreinte en base (ça empêcherait de repérer
/// des mots de passe réutilisés en cas de fuite de la base).
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| PasswordError::HashingFailed)
}

/// Vérifie un mot de passe en clair contre une empreinte PHC stockée.
///
/// Renvoie `Ok(false)` (pas une erreur) pour un mot de passe qui ne
/// correspond pas : un mot de passe incorrect est un résultat normal
/// de cette fonction, pas un dysfonctionnement — seule une empreinte
/// stockée corrompue ou dans un format inattendu renvoie `Err`.
pub fn verify_password(password: &str, stored_hash: &str) -> Result<bool, PasswordError> {
    let parsed_hash =
        PasswordHash::new(stored_hash).map_err(|_| PasswordError::InvalidHashFormat)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifie_le_bon_mot_de_passe() {
        let hash = hash_password("un mot de passe correct").unwrap();
        assert!(verify_password("un mot de passe correct", &hash).unwrap());
    }

    #[test]
    fn rejette_un_mauvais_mot_de_passe() {
        let hash = hash_password("un mot de passe correct").unwrap();
        assert!(!verify_password("un autre mot de passe", &hash).unwrap());
    }

    #[test]
    fn deux_hachages_du_meme_mot_de_passe_different() {
        // Sel aléatoire à chaque appel : propriété de sécurité
        // importante (cf. doc de `hash_password`), testée
        // explicitement pour qu'une régression future (ex: un sel fixe
        // introduit par erreur) soit détectée.
        let hash1 = hash_password("identique").unwrap();
        let hash2 = hash_password("identique").unwrap();
        assert_ne!(hash1, hash2);
        // ... mais les deux vérifient correctement le même mot de passe.
        assert!(verify_password("identique", &hash1).unwrap());
        assert!(verify_password("identique", &hash2).unwrap());
    }

    #[test]
    fn refuse_une_empreinte_corrompue() {
        let result = verify_password("peu importe", "ceci-nest-pas-un-hash-phc-valide");
        assert!(matches!(result, Err(PasswordError::InvalidHashFormat)));
    }
}
