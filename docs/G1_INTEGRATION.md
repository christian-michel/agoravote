# Intégration Ğ1v2 — recherche, architecture, statut

Ce document rassemble tout ce qui concerne le module d'identité Ğ1
optionnel (`crates/agoravote-g1`, cf. son `README.md` pour son statut
particulier de crate exclu du workspace principal) : ce qui a été
vérifié, comment le protocole est conçu, et ce qu'il reste à faire.

## 1. Ce qui a été vérifié (recherche, pas supposition)

- **Version du réseau** : la Ğ1v2 (réécriture Substrate/Polkadot-SDK
  de Duniter) est en production depuis le **7 mars 2026**, migrée
  depuis la Ğ1v1 (dernier bloc v1 : 913 638). Les deux versions sont
  incompatibles à tous les niveaux (protocole, API) — v1 utilisait une
  API HTTP/REST (« BMA »), v2 utilise du JSON-RPC Substrate
  (WebSocket) plus un indexeur GraphQL séparé (`duniter-squid`).
  **Toute documentation/tutoriel mentionnant `g1.duniter.org` avec une
  API BMA est de la v1 et ne doit pas être utilisée pour ce module.**
- **Pallets confirmés** (lecture directe de
  `docs/api/runtime-calls.md` du dépôt `duniter/duniter-v2s`, consultée
  le 10 septembre 2026) : `Identity` (création/confirmation
  d'identité), `Certification` (certifications de la toile de
  confiance), `Distance` (règle de distance, condition d'adhésion),
  `UniversalDividend` (`claim_uds` — le DU doit être réclamé
  explicitement, il ne s'accumule pas automatiquement sur le solde).
- **Accès réseau depuis cet environnement de développement : testé et
  bloqué.** `curl` vers `rpc.duniter.org` et `g1-squid.axiom-team.fr`
  renvoie explicitement `Host not in allowlist`. Aucune requête réelle
  contre le réseau Ğ1v2 n'a donc pu être exécutée pendant ce travail —
  contrairement à PostgreSQL, installé et testé localement sans
  problème (cf. `docs/DEVLOG.md`, itération 2).
- **Compilation locale : tentée sérieusement, bloquée.** Deux sessions
  de bisection de dépendances (même méthode que pour `sqlx`, avec plus
  de succès partiel) ont buté sur une exigence de compilateur (rustc
  1.76+) venant d'une dépendance transitive non identifiable sans
  résolution complète. Détail complet dans le `Cargo.toml` de
  `agoravote-g1`.

## 2. Principe du protocole retenu

Deux problèmes distincts, jamais mélangés (cf. le tableau déjà partagé
en conversation) :

| Étape | Question | Où dans le code |
|---|---|---|
| 1. Défi | Générer un texte aléatoire à signer | `challenge.rs` — **écrit, compilé, testé, fonctionne** |
| 2. Preuve de possession | Vérifier qu'une signature correspond à la clé publique annoncée | `signature.rs` — écrit, non testé (dépendance non compilable ici) |
| 3. Vérification d'adhésion | Interroger la chaîne : ce compte est-il membre actif ? | `chain.rs` — écrit, non testé (dépendance non compilable ici + pas d'accès réseau) |

**La phrase de 12 mots ne transite jamais vers AgoraVote.** Le
portefeuille de l'utilisateur (Cesium², Ğecko, extension navigateur)
signe le défi localement ; seul le résultat signé est envoyé. C'est
un principe non négociable, documenté en tête de `signature.rs`.

## 3. Ce qui est réellement vérifié aujourd'hui

```bash
cargo test -p agoravote-g1 --lib challenge
# 3 tests, tous verts — génération de défi, expiration, unicité
```

C'est peu au regard de l'ambition du module, mais c'est du solide :
aucune ligne testée n'a été présentée comme plus fiable qu'elle ne
l'est.

## 4. Sketch d'intégration dans `agoravote-api` (non câblé, à titre d'exemple)

Ce qui suit décrit la forme que prendrait l'intégration, **une fois**
`agoravote-g1` compilé et vérifié (cf. son `README.md`, section
« Pour le rendre utilisable »). Ce code n'existe pas dans
`agoravote-api` aujourd'hui — l'ajouter maintenant romprait la
compilation du workspace principal (cf. §1 de ce document).

### Nouvelles routes

| Route | Rôle |
|---|---|
| `POST /auth/g1/challenge` | Génère un [`agoravote_g1::Challenge`] et le renvoie au client |
| `POST /auth/g1/verify` | Reçoit `{public_key, signature}`, vérifie la signature, interroge la chaîne, crée une session (même mécanisme que `agoravote-auth`) |

### Esquisse du second handler

```rust
async fn g1_verify(
    State(state): State<AppState>,
    Json(req): Json<G1VerifyRequest>, // { public_key_hex, signature_hex, challenge_message }
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    let public_key = agoravote_g1::signature::decode_hex(&req.public_key_hex)
        .map_err(|_| bad_request("clé publique invalide"))?;
    let signature = agoravote_g1::signature::decode_hex(&req.signature_hex)
        .map_err(|_| bad_request("signature invalide"))?;

    let valid = agoravote_g1::verify_signature(&public_key, req.challenge_message.as_bytes(), &signature)
        .map_err(|_| bad_request("format de preuve invalide"))?;
    if !valid {
        return Err(unauthorized_login()); // même message anti-énumération que /auth/login
    }

    // Optionnel selon la campagne (§13) : exiger un compte membre actif.
    let membership = agoravote_g1::chain::check_membership(&state.g1_rpc_url, &public_key[..32].try_into().unwrap())
        .await
        .map_err(internal_error_g1)?;
    if !membership.is_member {
        return Err(bad_request("ce compte Ğ1 n'est pas membre actif de la toile de confiance"));
    }

    // Retrouver ou créer le User/Account associé à cette clé publique
    // Ğ1, puis émettre une session — même mécanisme que
    // `agoravote_auth::issue_session`, réutilisé tel quel.
    // ...
}
```

Notez ce que ce sketch NE fait PAS : il ne fait jamais confiance au
`voter_id` fourni par le client sans vérification (même principe déjà
appliqué dans `cast_ballot`, cf. `docs/SECURITY.md` §7), et il ne
stocke jamais la clé privée ni la phrase de 12 mots — seule la clé
**publique** (qui est par nature publique) est conservée, associée au
`User` comme un second moyen d'authentification possible.

### Modèle de données à ajouter

Suivant le même principe que `agoravote_core::auth::Account`
(§13) : un nouveau type `G1Link { user_id: Id, public_key: String,
linked_at: DateTime<Utc> }` dans `agoravote-core`, persisté par
`agoravote-store` dans une nouvelle table, indexée sur `public_key`
(unique — un compte Ğ1 ne peut être lié qu'à un seul utilisateur
AgoraVote, même logique que la contrainte d'unicité d'email déjà en
place).

## 5. Ce qui reste à faire, dans l'ordre

1. Compiler `agoravote-g1` dans un environnement à toolchain Rust à
   jour ; corriger les éventuelles erreurs d'API `subxt`/`subxt-signer`.
2. Confirmer les noms exacts de stockage (`Identity::IdentityIndexOf`,
   pallet `Membership` ou équivalent) contre un nœud `gdev` réel —
   **jamais `g1` en premier**.
3. Ajouter un test de signature avec un vecteur sr25519 connu.
4. Réintégrer le crate dans le workspace principal, vérifier que
   `cargo test --workspace` passe toujours intégralement.
5. Implémenter le sketch du §4 ci-dessus dans `agoravote-api`, avec
   les mêmes standards que le reste du projet : tests d'intégration,
   `clippy -D warnings`, test de bout en bout contre un vrai réseau
   `gdev`.
6. Documenter dans `docs/SECURITY.md` les implications spécifiques
   (le RPC configuré doit être fiable — un nœud malveillant pourrait
   mentir sur le statut de membre ; envisager d'interroger plusieurs
   nœuds indépendants pour une décision aussi sensible).

## 6. Références

- Duniter v2 est lancé (annonce officielle) —
  <https://duniter.fr/blog/duniter-v2/>
- Documentation des appels runtime (pallets confirmés) —
  <https://github.com/duniter/duniter-v2s/blob/master/docs/api/runtime-calls.md>
- Dépôt du runtime — <https://git.duniter.org/nodes/rust/duniter-v2s>
- `subxt` (client Rust Substrate, maintenu en fork par Duniter) —
  <https://github.com/duniter/subxt>
