//! Vérification d'adhésion à la toile de confiance Ğ1v2 — requête
//! réseau vers un nœud Duniter v2.
//!
//! ⚠️ **Le code de ce fichier n'a été ni compilé ni testé** — cf.
//! l'avertissement de `lib.rs`. Ce qui SUIT a été vérifié par lecture
//! directe de la documentation générée du runtime Duniter v2
//! (`docs/api/runtime-calls.md` du dépôt `duniter/duniter-v2s`,
//! consultée le 10 septembre 2026) :
//!
//! - Le réseau `g1` (production) compte **17 pallets à appels
//!   utilisateur**, dont **`Identity`** (création/confirmation
//!   d'identité, `owner_key: AccountId`, `IdtyIndex` comme index
//!   interne distinct du compte), **`Certification`** (`add_cert`,
//!   `renew_cert`), **`Distance`** (évaluation de la règle de
//!   distance, condition d'adhésion) et **`UniversalDividend`**
//!   (`claim_uds` — le DU n'est **pas** versé automatiquement, un
//!   appel explicite est nécessaire pour matérialiser le solde).
//! - **Aucun pallet nommé `Membership` n'apparaît** dans cette liste
//!   de 17 — mais cette documentation ne couvre que les *appels*
//!   (extrinsèques), pas le *stockage* : un pallet purement interne
//!   (le statut d'adhésion changeant en conséquence automatique d'une
//!   évaluation de distance réussie, plutôt que par appel direct de
//!   l'utilisateur) n'y apparaîtrait pas. Le nom `Membership` utilisé
//!   ci-dessous reste donc une **hypothèse**, pas une donnée confirmée
//!   — cf. section suivante.
//!
//! Conséquence de conception (répond directement à la question initiale
//! de l'utilisateur — "un compte qui produit un DU régulièrement") :
//! puisque le DU s'accumule pour tout membre sans dépendre de son
//! comportement de réclamation, ce module vérifie le **statut
//! d'adhésion courant**, pas un historique de réclamation — d'après la
//! conception du protocole, "membre actif" et "compte éligible au DU"
//! sont équivalents.
//!
//! Ce qui n'a PAS pu être vérifié et doit l'être avant toute mise en
//! production :
//! - Les noms EXACTS des éléments de stockage (`IdentityIndexOf`,
//!   `Identities`, l'existence et le nom d'un pallet `Membership`
//!   séparé) : non documentés publiquement au moment de la rédaction
//!   (le wiki officiel marque cette page "TODO"). Confirmer par
//!   introspection de la métadonnée d'un nœud réel (`subxt metadata`,
//!   ou `state_getMetadata` en RPC brut) contre `gdev` d'abord.
//! - La version exacte du runtime déployée sur le réseau `g1` de
//!   production au moment de l'utilisation (spec_version), qui peut
//!   avoir évolué depuis la rédaction de ce fichier.
//! - Le préfixe SS58 de la Ğ1 (nécessaire uniquement si vous affichez
//!   des adresses lisibles ; non nécessaire pour les fonctions
//!   ci-dessous, qui travaillent sur les octets bruts de la clé).

use subxt::dynamic::Value;
use subxt::{OnlineClient, PolkadotConfig};

use crate::error::G1Error;

/// Résultat d'une vérification d'adhésion.
#[derive(Debug, Clone)]
pub struct MembershipStatus {
    /// `true` si le compte est actuellement un membre actif de la
    /// toile de confiance (donc éligible au Dividende Universel,
    /// cf. doc de ce module).
    pub is_member: bool,
    /// Index d'identité interne (`IdtyIndex`) si une identité existe
    /// pour ce compte, indépendamment de son statut d'adhésion.
    pub identity_index: Option<u32>,
}

/// Se connecte à un nœud Duniter v2 et vérifie le statut d'adhésion
/// d'un compte, identifié par les octets bruts de sa clé publique
/// sr25519 (32 octets — les mêmes octets que pour
/// [`crate::signature::verify_signature`] ; c'est la même clé qui
/// prouve la possession ET identifie le compte sur la chaîne).
///
/// `rpc_url` : URL d'un nœud RPC Ğ1v2, ex.
/// `"wss://rpc.g1.duniter.org:443"` pour la production, ou une URL
/// `gdev` pour les tests — **toujours tester contre `gdev` avant
/// `g1`** (cf. §1 de ce fichier : réseau de test vs comptes réels).
///
/// # Erreurs
/// `Err(G1Error::Chain(..))` pour tout échec réseau ou de requête —
/// le message détaillé est fait pour les logs serveur
/// (`tracing::error!`), pas pour être renvoyé tel quel à un client
/// HTTP (même principe que `agoravote-api::routes::internal_error`).
pub async fn check_membership(
    rpc_url: &str,
    account_public_key: &[u8; 32],
) -> Result<MembershipStatus, G1Error> {
    let client = OnlineClient::<PolkadotConfig>::from_url(rpc_url)
        .await
        .map_err(|e| {
            tracing::error!(erreur = %e, url = rpc_url, "connexion au nœud Ğ1v2 échouée");
            G1Error::Chain(e.to_string())
        })?;

    let account_value = Value::from_bytes(account_public_key);

    // Étape 1 : compte -> index d'identité. Nom de storage à confirmer
    // (cf. avertissement en tête de fichier) — "IdentityIndexOf" est
    // le nom déduit du code observé (`IdentityAccountIdProvider`,
    // `IdentityIndexOf<T>` comme type de conversion), pas confirmé
    // comme nom de storage on-chain exact.
    let index_query = subxt::dynamic::storage("Identity", "IdentityIndexOf", vec![account_value]);

    let storage = client.storage().at_latest().await.map_err(|e| {
        tracing::error!(erreur = %e, "lecture de l'état de la chaîne échouée");
        G1Error::Chain(e.to_string())
    })?;

    let Some(index_result) = storage.fetch(&index_query).await.map_err(|e| {
        tracing::error!(erreur = %e, "requête IdentityIndexOf échouée");
        G1Error::Chain(e.to_string())
    })?
    else {
        // Aucune identité pour ce compte : ni membre, ni identité.
        return Ok(MembershipStatus {
            is_member: false,
            identity_index: None,
        });
    };

    let identity_index: u32 = index_result
        .to_value()
        .map_err(|e| G1Error::Chain(format!("décodage IdtyIndex : {e}")))?
        .as_u128()
        .ok_or_else(|| G1Error::Chain("IdtyIndex n'est pas un entier".to_string()))?
        as u32;

    // Étape 2 : index d'identité -> statut d'adhésion. L'existence et
    // le nom d'un pallet "Membership" séparé ne sont PAS confirmés
    // (cf. avertissement en tête de fichier) — c'est l'hypothèse la
    // plus probable compte tenu des conventions Substrate usuelles et
    // de la séparation conceptuelle identité/adhésion documentée dans
    // l'addendum v0.3, mais elle doit être vérifiée contre la
    // métadonnée réelle avant toute utilisation.
    let membership_query = subxt::dynamic::storage(
        "Membership",
        "Membership",
        vec![Value::u128(u128::from(identity_index))],
    );

    let membership_result = storage.fetch(&membership_query).await.map_err(|e| {
        tracing::error!(erreur = %e, "requête Membership échouée");
        G1Error::Chain(e.to_string())
    })?;

    Ok(MembershipStatus {
        is_member: membership_result.is_some(),
        identity_index: Some(identity_index),
    })
}
