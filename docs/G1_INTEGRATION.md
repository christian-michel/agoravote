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
# état serveur), signature (dont un vecteur ed25519 connu — cf. la note
# ci-dessous sur le correctif sr25519 -> ed25519, itération 9)

cargo test -p agoravote-api
# 17 tests, tous verts, dont 4 tests g1_* utilisant de VRAIES
# signatures ed25519 (ed25519-zebra en dev-dependency), pas des mocks
```

**⚠️ Correctif de schéma cryptographique (itération 9)** : jusqu'à
l'itération 8, ce protocole vérifiait des signatures **sr25519**, sur
la base d'une hypothèse jamais vérifiée. Des recherches web ont montré
que les comptes/portefeuilles Ğ1v2 réels (Ğecko, Cesium²) signent
probablement en **ed25519** (comme la Ğ1 historique), pas en sr25519
(spécifique à l'écosystème Polkadot) — cf. `docs/DEVLOG.md` itération
9 et `docs/SECURITY.md` §8 pour le détail complet, y compris la limite
de confiance de cette correction elle-même (non confirmée contre une
source primaire, `forum.duniter.org`/`git.duniter.org` restant
bloqués depuis cet environnement).

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
   ne l'a). Bloquant pour la suite de `chain.rs` spécifiquement — **plus
   bloquant pour obtenir une vérification d'adhésion à la toile de
   confiance en général**, cf. §7 ci-dessous (`sso-connect` offre une
   voie alternative qui ne dépend pas de cette confirmation).
3. ~~Ajouter un test de signature avec un vecteur cryptographique
   connu~~ — fait (itération 6, corrigé sr25519 -> ed25519 en
   itération 9, cf. ci-dessus).
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
8. ~~Client OAuth2 `sso.rs` pour `sso-connect`~~ — fait (itération 16,
   cf. §7 ci-dessous). Reste : câblage HTTP dans `agoravote-api`
   (routes, persistance, cookie `state`), qui dépend d'un domaine
   public pour AgoraVote (nécessaire à l'inscription manuelle auprès de
   l'opérateur de l'instance visée) — cf. §7.

## 6. Références

- Duniter v2 est lancé (annonce officielle) —
  <https://duniter.fr/blog/duniter-v2/>
- Documentation des appels runtime (pallets confirmés) —
  <https://github.com/duniter/duniter-v2s/blob/master/docs/api/runtime-calls.md>
- Dépôt du runtime — <https://git.duniter.org/nodes/rust/duniter-v2s>
- `subxt` (client Rust Substrate, maintenu en fork par Duniter) —
  <https://github.com/duniter/subxt>
- `sso-connect` (protocole OAuth2 pour une connexion Ğ1 via
  portefeuille, cf. §7) — <https://git.duniter.org/clients/sso-connect>,
  déploiement de référence <https://connect.monnaie-libre.fr>

## 7. `sso-connect` — troisième voie de connexion Ğ1, itération 16

Le porteur de projet a signalé l'existence de `sso-connect`
(<https://git.duniter.org/clients/sso-connect>), un serveur OAuth2 tiers
qui authentifie un utilisateur via son portefeuille Ğ1 (Gecko ≥ 1.6.0,
lien profond `g1://sso-login` ou QR code) **et** vérifie lui-même son
adhésion à la toile de confiance avant de renvoyer un document
d'identité — contournant ainsi, pour cette garantie précise, le
blocage réseau qui empêche `chain.rs` d'être vérifié depuis cet
environnement (§1, §5 point 2).

### Ce qui distingue cette voie du protocole défi/signature existant

| | `/auth/g1/challenge`+`/verify` (existant) | `sso-connect` (nouveau) |
|---|---|---|
| Prouve | Possession de clé uniquement | Possession de clé **+** adhésion actuelle à la toile de confiance |
| Dépendance réseau de notre serveur | Aucune (vérification ed25519 locale) | Deux appels HTTPS vers l'instance `sso-connect` |
| Notre code lit la chaîne Ğ1 | Non | Non — délégué à l'opérateur de l'instance `sso-connect` |
| Portefeuille requis | N'importe lequel (copier-coller manuel) | Gecko ≥ 1.6.0 ou tout portefeuille listant l'hôte `sso-connect` visé |

Les deux mécanismes sont **complémentaires, pas redondants** : le
premier ne dépend d'aucun tiers mais ne prouve jamais l'adhésion à la
toile de confiance (limite documentée depuis l'itération 8) ; le second
obtient cette preuve, au prix d'une dépendance à la disponibilité et à
l'honnêteté de l'opérateur de l'instance `sso-connect` choisie (cf.
`docs/SECURITY.md`, à mettre à jour lors du câblage HTTP — le nœud
Duniter et l'indexeur ne sont plus interrogés par nous, mais la
confiance qu'on leur accordait se reporte sur l'opérateur de
`sso-connect`).

### Ce qui a été fait (itération 16) : le client OAuth2, pas le câblage

`crates/agoravote-g1/src/sso.rs` (feature `sso-connect`, désactivée par
défaut) implémente le protocole documenté par `SITE-INTEGRATION.md` du
dépôt (fourni par le porteur de projet, pas deviné) :

- `SsoConfig::authorize_url` — construit l'URL de redirection vers
  `/oauth/authorize` (étape 1).
- `exchange_code` — `POST /oauth/token` (Basic auth, `grant_type=
  authorization_code`), renvoie un jeton d'accès opaque.
- `fetch_identity` — `GET /oauth/userinfo` (Bearer), renvoie
  `G1Identity { id, username, name, address, member, groups }`.
- `complete_login` — enchaîne les deux appels ci-dessus, cas d'usage
  normal pour l'appelant.

**Ce module ne génère ni ne vérifie le paramètre `state`** (protection
anti-CSRF requise par le protocole, cf. `SITE-INTEGRATION.md` §3 :
« with no PKCE, this is your defence against a forged callback ») :
cette responsabilité appartient à l'appelant HTTP (`agoravote-api`),
pas à ce crate qui ne fait aucune I/O HTTP entrante ni gestion de
session — cohérent avec la séparation des couches du projet.

**Statut de vérification** : 8 tests contre un vrai serveur HTTP local
(`wiremock`, port éphémère) reproduisant exactement les réponses
documentées — y compris un test qui vérifie l'en-tête `Authorization:
Basic ...` réellement envoyé, pas seulement le contenu du corps de
requête. **Jamais vérifié contre `connect.monnaie-libre.fr` ni aucun
déploiement réel** : cet environnement n'a toujours pas d'accès réseau
à ce domaine (même blocage que pour `chain.rs`, cf. §1).

### Ce qui reste à faire avant un câblage HTTP dans `agoravote-api`

1. **Un domaine public HTTPS pour AgoraVote.** L'inscription auprès de
   l'opérateur d'une instance (manuelle, cf. `SITE-INTEGRATION.md` §1)
   exige une URL de callback exacte, comparée octet à octet — rien à
   enregistrer tant qu'AgoraVote ne tourne qu'en local.
2. **L'inscription elle-même** : contacter l'opérateur (pour
   `connect.monnaie-libre.fr`, via `forum.monnaie-libre.fr`) avec l'URL
   de callback, un `client_id` souhaité, l'URL publique du site — reçoit
   en retour un `client_secret` (≥ 32 caractères) à stocker côté
   serveur uniquement (jamais dans le code, jamais côté client).
3. **Nouvelles routes `agoravote-api`** : une route de départ
   (redirection vers `authorize_url`, avec génération d'un `state`
   aléatoire posé en cookie `HttpOnly`/`Secure` à courte durée de vie —
   double-soumission cookie, sans avoir besoin d'état côté serveur,
   même philosophie que `Challenge::verify_freshness`) et une route de
   callback (vérifie `state` contre le cookie **avant toute chose**,
   appelle `complete_login`, retrouve ou provisionne le compte).
4. **Persistance** : l'identité renvoyée par `sso-connect` se clé sur
   `id` (`idty:<index>`), jamais sur `address` (cf.
   `SITE-INTEGRATION.md` : stable même après rotation de clé du
   portefeuille) — un nouveau lien, distinct de `G1Link`
   (clé sur `public_key_hex`, protocole défi/signature), donc une
   nouvelle table/migration plutôt qu'une réutilisation forcée d'un
   modèle qui répond à une question différente.
5. **Frontend** : un second bouton "Se connecter avec Ğ1 (SSO)" à côté
   du parcours `/connexion-g1` existant, avec la même honnêteté déjà
   appliquée à ce dernier sur ce que chaque mécanisme prouve
   réellement.
6. **Vérification de bout en bout réelle** avant toute mise en
   production, une fois 1-3 possibles (cf. CLAUDE.md, jamais de code
   non testé présenté comme fonctionnel).
