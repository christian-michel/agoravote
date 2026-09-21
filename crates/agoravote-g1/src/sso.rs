//! Client OAuth2 pour un serveur `sso-connect` tiers
//! (<https://git.duniter.org/clients/sso-connect>) — deuxième voie de
//! connexion Ğ1v2, complémentaire de [`crate::challenge`]/
//! [`crate::signature`] (cf. addendum v0.3 du cahier des charges).
//!
//! `sso-connect` est un serveur OAuth2 « authorization code » RFC 6749
//! **ordinaire** (pas un protocole maison) : un utilisateur approuve la
//! connexion dans son portefeuille Ğ1 (Gecko et compatibles, via un
//! lien profond `g1://sso-login` ou un QR code), et `sso-connect` lit
//! lui-même la chaîne Ğ1v2 pour confirmer son adhésion à la toile de
//! confiance avant de nous renvoyer un document d'identité. Ce module
//! ne fait donc AUCUNE cryptographie Ğ1 ni requête à un nœud Duniter —
//! contrairement à [`crate::chain`], qui tente de lire la chaîne
//! nous-mêmes (et reste non vérifiable dans cet environnement) : ici,
//! c'est `sso-connect` qui porte cette responsabilité, et nous ne
//! sommes qu'un client OAuth2 HTTPS parlant à un service tiers.
//!
//! Protocole complet tel que documenté par `SITE-INTEGRATION.md` du
//! dépôt (« start here to add Ğ1 sign-in to a site ») :
//!
//! 1. Rediriger le navigateur vers [`SsoConfig::authorize_url`].
//! 2. L'utilisateur approuve dans son portefeuille ; `sso-connect`
//!    redirige le navigateur vers notre `redirect_uri` enregistrée,
//!    avec `?code=...&state=...`.
//! 3. L'appelant DOIT vérifier `state` avant toute chose (défense
//!    contre un callback forgé — `sso-connect` n'implémente pas PKCE,
//!    cf. `SITE-INTEGRATION.md` §3 : « with no PKCE, this is your
//!    defence against a forged callback »). Ce module ne génère ni ne
//!    vérifie `state` lui-même : c'est à l'appelant (`agoravote-api`)
//!    de le lier à la session du navigateur (ex. cookie), aucune
//!    logique HTTP de session n'appartenant à ce crate.
//! 4. [`exchange_code`] échange le code contre un jeton d'accès.
//! 5. [`fetch_identity`] lit le document d'identité avec ce jeton.
//!    [`complete_login`] enchaîne 4 et 5 en un seul appel, cas d'usage
//!    normal côté appelant.
//!
//! ## Statut de vérification
//!
//! Testé (cf. `mod tests` ci-dessous) contre un vrai serveur HTTP local
//! (port éphémère, requêtes/réponses HTTP réelles via `wiremock`) qui
//! reproduit exactement les formats documentés par `SITE-INTEGRATION.md`
//! (`/oauth/token`, `/oauth/userinfo`, formes d'erreur JSON) — **jamais
//! vérifié contre un déploiement réel** (`connect.monnaie-libre.fr`
//! reste hors d'atteinte réseau depuis tout environnement de
//! développement utilisé jusqu'ici pour ce projet, cf. `lib.rs`). À
//! revérifier en conditions réelles avant mise en production, une fois
//! AgoraVote enregistré comme client OAuth2 (inscription manuelle
//! auprès de l'opérateur de l'instance visée, cf.
//! `SITE-INTEGRATION.md` §1 : « Registration is manual »).
//!
//! ## Ce que garantit ce module, et ce qu'il ne garantit PAS
//!
//! Une identité renvoyée par [`fetch_identity`]/[`complete_login`]
//! prouve la possession de clé **et** l'adhésion à la toile de
//! confiance **au moment de cette connexion précise** — `member` est
//! un instantané, pas un abonnement (cf. `SITE-INTEGRATION.md` §6 :
//! « Treat `member` as a snapshot »). Un compte qui perd sa qualité de
//! membre après coup n'en est pas informé automatiquement : si
//! l'appartenance conditionne un accès côté AgoraVote, elle doit être
//! relue à chaque connexion, jamais mise en cache indéfiniment.

use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};

use crate::error::G1Error;

/// Un enregistrement client OAuth2 obtenu manuellement auprès de
/// l'opérateur d'une instance `sso-connect` (cf.
/// `SITE-INTEGRATION.md` §1, "Get registered" : pas d'inscription en
/// libre-service). Ce type ne fait aucune hypothèse sur l'instance
/// visée : `base_url` peut pointer vers `connect.monnaie-libre.fr` ou
/// vers toute autre instance conforme au même protocole.
#[derive(Debug, Clone)]
pub struct SsoConfig {
    /// URL de base de l'instance `sso-connect`, sans slash final
    /// (ex. `https://connect.monnaie-libre.fr`).
    pub base_url: String,
    pub client_id: String,
    pub client_secret: String,
    /// Doit correspondre OCTET POUR OCTET à l'URL enregistrée auprès
    /// de l'opérateur — `SITE-INTEGRATION.md` §1 : « compared byte for
    /// byte, must be https, never extended by prefix ».
    pub redirect_uri: String,
}

impl SsoConfig {
    fn token_endpoint(&self) -> String {
        format!("{}/oauth/token", self.base_url)
    }

    fn userinfo_endpoint(&self) -> String {
        format!("{}/oauth/userinfo", self.base_url)
    }

    /// Construit l'URL vers laquelle rediriger le navigateur — étape 1
    /// du protocole (`SITE-INTEGRATION.md` §3, "Step 1"). Pure : ne
    /// fait aucune requête réseau elle-même, seul le navigateur de
    /// l'utilisateur contacte `sso-connect` à cette étape, jamais notre
    /// serveur.
    ///
    /// `state` doit être généré et vérifié par l'appelant (cf.
    /// documentation du module ci-dessus) : ce module se contente de
    /// le porter tel quel dans l'URL.
    pub fn authorize_url(&self, state: &str) -> Result<String, G1Error> {
        let mut url = Url::parse(&format!("{}/oauth/authorize", self.base_url))
            .map_err(|e| G1Error::Sso(format!("URL de base sso-connect invalide : {e}")))?;
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("state", state);
        Ok(url.to_string())
    }
}

/// Réponse de `POST /oauth/token` — cf. `SITE-INTEGRATION.md` §3,
/// "Step 4". Jeton opaque, jamais un JWT (« There is no discovery
/// document, no JWKS and no ID token » — `SITE-INTEGRATION.md` §2) :
/// il ne sert qu'à lire `/oauth/userinfo` une seule fois, pas à porter
/// lui-même une identité déchiffrable.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccessToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

/// Document d'identité renvoyé par `GET /oauth/userinfo` — cf.
/// `SITE-INTEGRATION.md` §"What your site receives". Champs et noms
/// JSON repris tels quels de cette doc, pas devinés.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct G1Identity {
    /// `idty:` suivi de l'index d'identité on-chain — **la seule clé
    /// de compte à utiliser côté AgoraVote** (cf. `SITE-INTEGRATION.md`
    /// §"Key the account on `id`, never on the address" : stable même
    /// après une rotation de clé du portefeuille, contrairement à
    /// `address`).
    pub id: String,
    /// Nom d'identité Ğ1, réservé et unique on-chain — bon nom
    /// d'affichage par défaut, mais PAS une clé (cf. §6 :
    /// « Do not authorise on `username` alone »).
    pub username: String,
    pub name: String,
    /// Adresse SS58 ayant signé cette connexion — affichage
    /// uniquement, jamais une preuve : « Anybody can quote somebody
    /// else's address » (§6).
    pub address: String,
    /// Adhésion à la toile de confiance **au moment de cette
    /// connexion** — un instantané, pas un abonnement (cf. doc du
    /// module ci-dessus).
    pub member: bool,
    #[serde(default)]
    pub groups: Vec<String>,
}

/// Corps JSON d'une erreur de protocole OAuth2, tel que renvoyé par
/// `/oauth/token` et `/oauth/userinfo` en cas d'échec (§4 : `{ "error":
/// "invalid_grant" }`). Les jetons d'erreur eux-mêmes
/// (`invalid_client`, `invalid_grant`, `rate_limited`,
/// `temporarily_unavailable`...) sont documentés par
/// `SITE-INTEGRATION.md` §4 — propagés tels quels dans
/// [`G1Error::Sso`] plutôt que ré-encodés dans une variante par
/// jeton : l'appelant qui a besoin de distinguer ces cas peut inspecter
/// le message, et aucune route d'`agoravote-api` n'a aujourd'hui besoin
/// d'un traitement différencié par jeton précis.
#[derive(Debug, Deserialize)]
struct SsoErrorBody {
    error: String,
}

/// Échange un code d'autorisation contre un jeton d'accès — étape 4 du
/// protocole (`SITE-INTEGRATION.md` §3). Authentification du client par
/// secret (Basic auth), pas par corps de requête : les deux formes sont
/// acceptées par `sso-connect` d'après la doc, Basic auth est la plus
/// largement supportée par les bibliothèques HTTP clientes.
pub async fn exchange_code(config: &SsoConfig, code: &str) -> Result<AccessToken, G1Error> {
    let client = Client::new();
    let response = client
        .post(config.token_endpoint())
        .basic_auth(&config.client_id, Some(&config.client_secret))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", config.redirect_uri.as_str()),
        ])
        .send()
        .await
        .map_err(|e| G1Error::Sso(format!("échange du code d'autorisation impossible : {e}")))?;

    parse_json_or_error(response).await
}

/// Lit le document d'identité associé à un jeton d'accès — étape 5 du
/// protocole (`SITE-INTEGRATION.md` §3). `GET`, pas `POST` : les deux
/// sont acceptés par `sso-connect` et renvoient le même document, `GET`
/// est le choix le plus ordinaire pour une lecture sans effet de bord.
pub async fn fetch_identity(config: &SsoConfig, access_token: &str) -> Result<G1Identity, G1Error> {
    let client = Client::new();
    let response = client
        .get(config.userinfo_endpoint())
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| G1Error::Sso(format!("lecture de l'identité impossible : {e}")))?;

    parse_json_or_error(response).await
}

/// Enchaîne l'échange du code puis la lecture de l'identité — ce que
/// `agoravote-api` doit normalement appeler depuis son callback OAuth2
/// une fois `state` vérifié (cf. doc du module ci-dessus).
pub async fn complete_login(config: &SsoConfig, code: &str) -> Result<G1Identity, G1Error> {
    let token = exchange_code(config, code).await?;
    fetch_identity(config, &token.access_token).await
}

/// Désérialise une réponse JSON en `T` si le statut HTTP est un
/// succès, sinon tente de lire le corps d'erreur `{"error": "..."}`
/// documenté (§4) et le propage dans [`G1Error::Sso`] — avec le texte
/// brut en repli si le corps n'a pas cette forme exacte (ex. panne
/// intermédiaire renvoyant du HTML), pour ne jamais avaler une erreur
/// silencieusement.
async fn parse_json_or_error<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, G1Error> {
    let status = response.status();
    if status.is_success() {
        response
            .json::<T>()
            .await
            .map_err(|e| G1Error::Sso(format!("réponse inattendue du fournisseur SSO : {e}")))
    } else {
        let body = response.text().await.unwrap_or_default();
        let reason = serde_json::from_str::<SsoErrorBody>(&body)
            .map(|b| b.error)
            .unwrap_or(body);
        Err(G1Error::Sso(format!("{status} {reason}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn config_for(server: &MockServer) -> SsoConfig {
        SsoConfig {
            base_url: server.uri(),
            client_id: "agoravote".to_string(),
            client_secret: "x".repeat(32),
            redirect_uri: "https://example.org/auth/g1/sso/callback".to_string(),
        }
    }

    #[test]
    fn authorize_url_contient_les_parametres_oauth2_attendus() {
        let config = SsoConfig {
            base_url: "https://connect.monnaie-libre.fr".to_string(),
            client_id: "agoravote".to_string(),
            client_secret: "x".repeat(32),
            redirect_uri: "https://example.org/auth/g1/sso/callback".to_string(),
        };
        let url = config.authorize_url("un-etat-aleatoire").unwrap();

        assert!(url.starts_with("https://connect.monnaie-libre.fr/oauth/authorize?"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=agoravote"));
        assert!(url.contains("state=un-etat-aleatoire"));

        // `redirect_uri` est ré-encodé par `Url::query_pairs_mut` : on
        // vérifie sa présence décodée plutôt que la forme exacte
        // encodée dans la chaîne brute.
        let parsed = Url::parse(&url).unwrap();
        let redirect = parsed
            .query_pairs()
            .find(|(k, _)| k == "redirect_uri")
            .expect("redirect_uri absent de l'URL construite")
            .1;
        assert_eq!(redirect, "https://example.org/auth/g1/sso/callback");
    }

    #[tokio::test]
    async fn exchange_code_analyse_une_reponse_valide() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "a".repeat(64),
                "token_type": "Bearer",
                "expires_in": 300
            })))
            .mount(&server)
            .await;

        let token = exchange_code(&config_for(&server), "un-code")
            .await
            .unwrap();

        assert_eq!(token.token_type, "Bearer");
        assert_eq!(token.expires_in, 300);
        assert_eq!(token.access_token.len(), 64);
    }

    #[tokio::test]
    async fn exchange_code_envoie_bien_lauthentification_et_les_champs_attendus() {
        // Vérifie qu'on envoie exactement ce que `SITE-INTEGRATION.md`
        // §3 "Step 4" documente : Basic auth + grant_type=
        // authorization_code + code + redirect_uri en formulaire — pas
        // une supposition sur la forme de la requête.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(header(
                "Authorization",
                format!("Basic {}", base64_basic_auth("agoravote", &"x".repeat(32))).as_str(),
            ))
            .and(body_string_contains("grant_type=authorization_code"))
            .and(body_string_contains("code=un-code"))
            .and(body_string_contains(
                "redirect_uri=https%3A%2F%2Fexample.org%2Fauth%2Fg1%2Fsso%2Fcallback",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "a".repeat(64),
                "token_type": "Bearer",
                "expires_in": 300
            })))
            .mount(&server)
            .await;

        exchange_code(&config_for(&server), "un-code")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn exchange_code_propage_le_jeton_derreur_invalid_grant() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(serde_json::json!({"error": "invalid_grant"})),
            )
            .mount(&server)
            .await;

        let err = exchange_code(&config_for(&server), "code-deja-utilise")
            .await
            .unwrap_err();

        assert!(matches!(&err, G1Error::Sso(msg) if msg.contains("invalid_grant")));
    }

    #[tokio::test]
    async fn fetch_identity_analyse_le_document_didentite() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/oauth/userinfo"))
            .and(header("Authorization", "Bearer un-jeton"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "idty:17758",
                "username": "Benham",
                "name": "Benham",
                "address": "g1LqPgmosBjJC4qW7iniiAxopFXVugVHPzhDPNKCasdT6UFJo",
                "member": true,
                "groups": ["g1-members"]
            })))
            .mount(&server)
            .await;

        let identity = fetch_identity(&config_for(&server), "un-jeton")
            .await
            .unwrap();

        assert_eq!(identity.id, "idty:17758");
        assert_eq!(identity.username, "Benham");
        assert!(identity.member);
        assert_eq!(identity.groups, vec!["g1-members".to_string()]);
    }

    #[tokio::test]
    async fn fetch_identity_gere_une_identite_non_membre_sans_groupe() {
        // `groups` est documenté comme `[]` pour un non-membre — vérifie
        // que le champ, absent ou vide, ne fait pas échouer le
        // désérialiseur (`#[serde(default)]`).
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/oauth/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "idty:99",
                "username": "nonmembre",
                "name": "nonmembre",
                "address": "g1test",
                "member": false
            })))
            .mount(&server)
            .await;

        let identity = fetch_identity(&config_for(&server), "un-jeton")
            .await
            .unwrap();

        assert!(!identity.member);
        assert!(identity.groups.is_empty());
    }

    #[tokio::test]
    async fn fetch_identity_refuse_un_jeton_invalide() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/oauth/userinfo"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(serde_json::json!({"error": "invalid_client"})),
            )
            .mount(&server)
            .await;

        let err = fetch_identity(&config_for(&server), "jeton-invalide")
            .await
            .unwrap_err();

        assert!(matches!(&err, G1Error::Sso(msg) if msg.contains("invalid_client")));
    }

    #[tokio::test]
    async fn complete_login_enchaine_lechange_du_code_puis_la_lecture_de_lidentite() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "jeton-de-test",
                "token_type": "Bearer",
                "expires_in": 300
            })))
            .mount(&server)
            .await;
        // Vérifie que le jeton reçu à l'étape précédente est bien celui
        // réutilisé pour lire l'identité, pas une valeur indépendante.
        Mock::given(method("GET"))
            .and(path("/oauth/userinfo"))
            .and(header("Authorization", "Bearer jeton-de-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "idty:1",
                "username": "test",
                "name": "test",
                "address": "g1test",
                "member": true,
                "groups": ["g1-members"]
            })))
            .mount(&server)
            .await;

        let identity = complete_login(&config_for(&server), "un-code")
            .await
            .unwrap();

        assert_eq!(identity.id, "idty:1");
    }

    /// Reproduit l'encodage Basic auth (RFC 7617) pour vérifier, dans
    /// le test ci-dessus, que `reqwest` envoie bien l'en-tête attendu —
    /// implémentation indépendante minimale plutôt qu'un import
    /// supplémentaire pour un seul test.
    fn base64_basic_auth(user: &str, password: &str) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let input = format!("{user}:{password}");
        let bytes = input.as_bytes();
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0];
            let b1 = *chunk.get(1).unwrap_or(&0);
            let b2 = *chunk.get(2).unwrap_or(&0);
            let n = (b0 as u32) << 16 | (b1 as u32) << 8 | (b2 as u32);
            out.push(ALPHABET[(n >> 18 & 0x3f) as usize] as char);
            out.push(ALPHABET[(n >> 12 & 0x3f) as usize] as char);
            out.push(if chunk.len() > 1 {
                ALPHABET[(n >> 6 & 0x3f) as usize] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                ALPHABET[(n & 0x3f) as usize] as char
            } else {
                '='
            });
        }
        out
    }
}
