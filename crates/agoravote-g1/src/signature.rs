//! Vérification de signature sr25519 — cœur cryptographique de la
//! preuve de possession de clé.
//!
//! **Principe de sécurité non négociable** : ce module ne reçoit et
//! ne manipule jamais que des clés publiques et des signatures. La
//! phrase de 12 mots (ou toute clé privée) d'un participant ne doit
//! **jamais** transiter vers AgoraVote, ni être traitée par ce
//! module ou par aucun autre — cf. l'échange précédent avec
//! l'utilisateur sur ce point. La signature est produite localement,
//! côté client, par le portefeuille de l'utilisateur.
//!
//! Compilé et testé (rustc 1.94, `cargo test -p agoravote-g1 --features
//! signature-verification`) — cf. `docs/DEVLOG.md` pour l'itération qui
//! a levé cette vérification, et `crates/agoravote-g1/README.md` pour
//! le statut à jour de ce crate.

use subxt_signer::sr25519::{self, PublicKey, Signature};

use crate::error::G1Error;

/// Longueur en octets d'une clé publique sr25519.
pub const PUBLIC_KEY_LEN: usize = 32;
/// Longueur en octets d'une signature sr25519.
pub const SIGNATURE_LEN: usize = 64;

/// Vérifie qu'une signature correspond bien au message et à la clé
/// publique fournis.
///
/// Cette fonction est volontairement **pure** : aucune I/O, aucun état
/// caché. À entrées identiques, elle renvoie toujours le même
/// résultat — condition nécessaire pour qu'une preuve d'identité soit
/// auditable (même principe que la pureté exigée de
/// `VotingMethod::tally`, cf. `agoravote-core`).
///
/// Renvoie `Ok(true)`/`Ok(false)` selon que la signature est valide ou
/// non — ce n'est PAS une erreur qu'une signature soit invalide (un
/// participant peut se tromper de clé, ou tenter une usurpation) ;
/// seule une entrée malformée (mauvaise longueur) renvoie `Err`.
pub fn verify_signature(
    public_key_bytes: &[u8],
    message: &[u8],
    signature_bytes: &[u8],
) -> Result<bool, G1Error> {
    let public_key: [u8; PUBLIC_KEY_LEN] = public_key_bytes
        .try_into()
        .map_err(|_| G1Error::InvalidPublicKey)?;
    let signature: [u8; SIGNATURE_LEN] = signature_bytes
        .try_into()
        .map_err(|_| G1Error::InvalidSignature)?;

    let public_key = PublicKey(public_key);
    let signature = Signature(signature);

    // Confirmé par compilation contre subxt-signer 0.37.0 réel (rustc
    // 1.94) : `verify` est une fonction libre du module `sr25519`, pas
    // une méthode de `PublicKey` (l'inverse avait été supposé faute de
    // pouvoir compiler dans l'environnement d'origine).
    Ok(sr25519::verify(&signature, message, &public_key))
}

/// Décode une chaîne hexadécimale (avec ou sans préfixe `0x`) en
/// octets. Utilitaire minimal pour éviter une dépendance
/// supplémentaire (crate `hex`) pour un besoin aussi simple — les
/// clés publiques et signatures transitent typiquement en hexadécimal
/// dans les API JSON des portefeuilles Ğ1.
pub fn decode_hex(input: &str) -> Result<Vec<u8>, G1Error> {
    let input = input.strip_prefix("0x").unwrap_or(input);
    if !input.len().is_multiple_of(2) {
        return Err(G1Error::InvalidSignature);
    }
    (0..input.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&input[i..i + 2], 16).map_err(|_| G1Error::InvalidSignature))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_hex_accepte_avec_et_sans_prefixe() {
        assert_eq!(decode_hex("0xff00").unwrap(), vec![0xff, 0x00]);
        assert_eq!(decode_hex("ff00").unwrap(), vec![0xff, 0x00]);
    }

    #[test]
    fn decode_hex_refuse_une_longueur_impaire() {
        assert!(decode_hex("abc").is_err());
    }

    #[test]
    fn verify_signature_refuse_une_cle_publique_de_mauvaise_taille() {
        let result = verify_signature(&[0u8; 10], b"message", &[0u8; SIGNATURE_LEN]);
        assert!(matches!(result, Err(G1Error::InvalidPublicKey)));
    }

    #[test]
    fn verify_signature_refuse_une_signature_de_mauvaise_taille() {
        let result = verify_signature(&[0u8; PUBLIC_KEY_LEN], b"message", &[0u8; 10]);
        assert!(matches!(result, Err(G1Error::InvalidSignature)));
    }

    // Vecteur de test connu : le compte de développement Substrate
    // standard "//Alice" (dérivation publique, documentée par
    // `subxt_signer::sr25519::dev`, jamais utilisée pour un vrai compte
    // Ğ1) — cf. README du crate, étape 3 ("ajouter un test avec un
    // vecteur sr25519 connu"), maintenant possible avec une toolchain
    // à jour.
    #[test]
    fn verify_signature_accepte_une_signature_valide_du_compte_de_dev_alice() {
        let alice = subxt_signer::sr25519::dev::alice();
        let message = b"AgoraVote - preuve de possession de compte";
        let signature = alice.sign(message);

        let result = verify_signature(&alice.public_key().0, message, &signature.0);

        assert!(matches!(result, Ok(true)));
    }

    #[test]
    fn verify_signature_refuse_une_signature_valide_pour_un_autre_message() {
        let alice = subxt_signer::sr25519::dev::alice();
        let signature = alice.sign(b"message original");

        let result = verify_signature(&alice.public_key().0, b"message modifie", &signature.0);

        assert!(matches!(result, Ok(false)));
    }
}
