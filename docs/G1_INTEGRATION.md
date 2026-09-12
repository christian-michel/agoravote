# Intégration Ğ1v2 — recherche, architecture, statut

Ce document rassemble tout ce qui concerne le module d'identité Ğ1
optionnel (`crates/agoravote-g1`, membre du workspace principal depuis
l'itération 8 — cf. son `README.md` pour le détail de cette
réintégration) : ce qui a été vérifié, comment le protocole est
conçu, et ce qu'il reste à faire.

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
| 1. Défi | Générer un texte aléatoire à signer, et vérifier sa fraîcheur sans état serveur | `challenge.rs` — **écrit, compilé, testé, câblé** (`POST /auth/g1/challenge`) |
| 2. Preuve de possession | Vérifier qu'une signature correspond à la clé publique annoncée | `signature.rs` — **écrit, compilé, testé, câblé** (`POST /auth/g1/verify`, feature `signature-verification`) |
| 3. Vérification d'adhésion | Interroger la chaîne : ce compte est-il membre actif ? | `chain.rs` — écrit, compile (feature `chain-query`) mais **non testé et non câblé à aucune route** (dépendance réseau non vérifiable ici) |

Cf. `docs/DEVLOG.md` itération 8 pour l'implémentation complète des
étapes 1 et 2, et `docs/SECURITY.md` §8 pour ce que cette
implémentation prouve — et ne prouve PAS (étape 3 non câblée).

**La phrase de 12 mots ne transite jamais vers AgoraVote.** Le
portefeuille de l'utilisateur (Cesium², Ğecko, extension navigateur)
signe le défi localement ; seul le résultat signé est envoyé. C'est
un principe non négociable, documenté en tête de `signature.rs`.

## 3. Ce qui est réellement vérifié aujourd'hui

```bash
cargo test -p agoravote-g1 --features chain-query
# 13 tests, tous verts — défi (génération, expiration, fraîcheur sans
# état serveur), signature (dont un vecteur sr25519 connu, //Alice)

cargo test -p agoravote-api
# 17 tests, tous verts, dont 4 tests g1_* utilisant de VRAIES
# signatures sr25519 (subxt-signer en dev-dependency), pas des mocks
```

Le protocole défi/signature (§2, étapes 1 et 2) est intégralement
câblé et testé. Seule l'étape 3 (vérification d'adhésion à la toile de
confiance) reste non vérifiée — cf. §4.

## 4. Intégration dans `agoravote-api` — câblée (itération 8)

`crates/agoravote-api/src/routes.rs` implémente les deux routes
suivantes (cf. `docs/openapi.yaml` pour le contrat complet), et
`crates/agoravote-api/Cargo.toml` dépend d'`agoravote-g1` avec
uniquement la feature `signature-verification` :

| Route | Rôle |
|---|---|
| `POST /auth/g1/challenge` | Génère un [`agoravote_g1::Challenge`] et le renvoie au client — sans état côté serveur, cf. `Challenge::verify_freshness` |
| `POST /auth/g1/verify` | Reçoit `{organization_id, public_key_hex, signature_hex, message}`, vérifie la signature, retrouve ou provisionne le `User`/`G1Link` associé, crée une session (même mécanisme que `agoravote-auth`) |

`g1_verify` (cf. sa doc dans `routes.rs`) : décode la clé publique et
la signature en hexadécimal, vérifie la fraîcheur du défi
(`Challenge::verify_freshness`) **puis** la signature
(`agoravote_g1::verify_signature`) sur le message réellement reçu —
jamais l'inverse ni l'une sans l'autre (cf. le test dédié dans
`challenge.rs` qui documente pourquoi). Ne fait jamais confiance au
`voter_id`/à l'identité déclarée par le client sans preuve (même
principe déjà appliqué dans `cast_ballot`, cf. `docs/SECURITY.md` §7),
et ne stocke jamais la clé privée ni la phrase de 12 mots — seule la
clé **publique** est conservée, via `G1Link` (`agoravote-core::auth`),
associée au `User` comme second moyen d'authentification possible,
jamais à la place d'`Account`.

**Volontairement absent de ce câblage : `chain.rs`/`chain-query`.**
`agoravote-api` n'active pas cette feature — cf. §1 ci-dessus et
`docs/SECURITY.md` §8 pour les deux raisons (une actuelle, une
durable) pour lesquelles cette vérification de toile de confiance
n'est pas branchée à `/auth/g1/verify`.

### Modèle de données (fait)

`G1Link { id: Id, user_id: Id, public_key_hex: String, linked_at:
DateTime<Utc> }` dans `agoravote-core::auth`, persisté par
`agoravote-store` dans la table `g1_links` (migration
`0003_g1_link.sql`), indexée `UNIQUE` sur `public_key_hex` — même
logique que la contrainte d'unicité d'email sur `Account`. Testé contre
un vrai PostgreSQL (`crates/agoravote-store/tests/postgres_integration.rs`).

### Frontend (fait)

Écran `/connexion-g1` (`frontend/src/pages/G1Login.tsx`, planche 17 de
l'addendum) : parcours manuel en deux étapes (générer un défi, coller
la clé publique et la signature obtenues d'un portefeuille externe —
aucune intégration d'extension de portefeuille pour l'instant). Copie
honnête sur ce que cette connexion prouve (possession de clé) et ne
prouve pas (appartenance à la toile de confiance).

## 5. Ce qui reste à faire, dans l'ordre

1. ~~Compiler `agoravote-g1` dans un environnement à toolchain Rust à
   jour~~ — fait (itération 6).
2. **Confirmer les noms exacts de stockage** (`Identity::IdentityIndexOf`,
   pallet `Membership` ou équivalent) contre un nœud `gdev` réel —
   **jamais `g1` en premier** — depuis un environnement qui a accès
   réseau à l'infrastructure Duniter (aucun de ceux utilisés jusqu'ici
   ne l'a). Bloquant pour la suite.
3. ~~Ajouter un test de signature avec un vecteur sr25519 connu~~ —
   fait (itération 6).
4. ~~Réintégrer le crate dans le workspace principal~~ — fait
   (itération 8).
5. ~~Implémenter le sketch du §4 ci-dessus dans `agoravote-api`~~ —
   fait pour `signature-verification` (itération 8). Reste, une fois
   l'étape 2 validée : câbler `chain-query`
   (`chain::check_membership`) comme vérification optionnelle
   supplémentaire sur `/auth/g1/verify`, avec les mêmes standards que
   le reste du projet (tests d'intégration, `clippy -D warnings`,
   test de bout en bout contre un vrai réseau `gdev`).
6. ~~Documenter dans `docs/SECURITY.md` les implications spécifiques~~
   — fait par anticipation (itération 8, §8 : le RPC configuré doit
   être fiable, un nœud malveillant pourrait mentir sur le statut de
   membre ; interroger plusieurs nœuds indépendants recommandé avant
   toute décision de légitimité de vote basée sur ce résultat).
7. Intégration d'une extension de portefeuille Ğ1 côté frontend, pour
   remplacer le copier-coller manuel actuel de `/connexion-g1`.

## 6. Références

- Duniter v2 est lancé (annonce officielle) —
  <https://duniter.fr/blog/duniter-v2/>
- Documentation des appels runtime (pallets confirmés) —
  <https://github.com/duniter/duniter-v2s/blob/master/docs/api/runtime-calls.md>
- Dépôt du runtime — <https://git.duniter.org/nodes/rust/duniter-v2s>
- `subxt` (client Rust Substrate, maintenu en fork par Duniter) —
  <https://github.com/duniter/subxt>
