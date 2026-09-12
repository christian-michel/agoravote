# Sécurité — revue effectuée et limites connues

## 1. Dépendances (analyse de vulnérabilités)

`cargo-audit` n'a pas pu être installé dans l'environnement de
développement utilisé pour cette revue (toolchain `rustc 1.75`
imposée par les paquets Ubuntu disponibles ; `cargo-audit` récent
exige `rustc >= 1.88`, et la compilation de la version compatible
0.21.1 a dépassé le temps disponible).

À défaut, une revue manuelle a été faite : clonage de la base
[RustSec advisory-db](https://github.com/RustSec/advisory-db) et
vérification de chaque crate résolu dans `Cargo.lock` contre les avis
existants. **Résultat : aucune vulnérabilité connue applicable** aux
versions actuellement utilisées. Détail des crates ayant un historique
d'avis (tous corrigés dans les versions utilisées ici) :

| Crate | Version utilisée | Avis vérifiés | Statut |
|---|---|---|---|
| tokio | 1.53.1 | RUSTSEC-2021-0072, 2021-0124, 2023-0001, 2023-0005, 2025-0023 | Toutes corrigées avant 1.53.1 |
| chrono | 0.4.45 | RUSTSEC-2020-0159 (segfault `localtime_r`) | Corrigé depuis 0.4.20 |
| hyper | 1.11.1 | RUSTSEC-2016 à 2022 (versions 0.x/legacy) | Sans objet, ces avis visent hyper < 1.0 |
| mio | 1.2.3 | RUSTSEC-2020-0081, 2024-0019 (Windows `NamedPipe` uniquement) | Sans objet (Linux) et corrigé |
| axum-core | 0.4.5 | RUSTSEC-2022-0055 (pas de limite de corps par défaut) | Corrigé depuis 0.2.8 — voir §2 ci-dessous |
| thread_local | 1.1.10 | RUSTSEC-2022-0006 (data race) | Corrigé depuis 1.1.4 |
| slab | 0.4.12 | RUSTSEC-2025-0047 | Corrigé depuis 0.4.11 |
| bytes | 1.12.1 | RUSTSEC-2026-0007 | Corrigé depuis 1.11.1 |
| http | 1.5.0 | RUSTSEC-2019-0033, 2019-0034 (versions 0.1.x) | Sans objet |
| socket2 | 0.6.5 | RUSTSEC-2020-0079 (versions < 0.3.16) | Sans objet |

**Recommandation pour la suite** : faire tourner `cargo audit` (ou
l'action GitHub `rustsec/audit-check`) dans la CI dès qu'une toolchain
plus récente que 1.75 est disponible dans l'environnement de build —
un job est prévu mais commenté dans `.github/workflows/ci.yml` en
attendant.

## 2. Durcissement HTTP appliqué

Cf. `crates/agoravote-api/src/routes.rs`, section `build_router` :

- **Limite de corps de requête** : axum applique déjà par défaut une
  limite de 2 Mo sur les corps consommés par `Json`/`String`/`Bytes`
  (c'est précisément le correctif historique de RUSTSEC-2022-0055,
  CVE-2022-3212 — vérifié dans la documentation d'axum-core). Ce
  projet la resserre à 64 Ko via `DefaultBodyLimit::max(64 * 1024)`,
  cohérent avec la taille réelle des charges utiles de cette API
  (une campagne, un bulletin — jamais un fichier).
- **Timeout de requête** : 10 secondes (`TimeoutLayer`), protection
  basique contre les clients lents ("slowloris").
- **Catch panic** : toute panique dans un handler devient une réponse
  500 loggée plutôt qu'une connexion coupée sans réponse
  (`CatchPanicLayer` — voir aussi `docs/LOGGING.md`).
- **Arrêt propre sur SIGTERM** : évite qu'un `docker stop` ou un
  redéploiement laisse des requêtes en cours sans réponse.

## 3. Robustesse du code (durcissement appliqué lors de la revue)

- Tous les accès au `Mutex` interne de l'API (`AppState`) passent par
  `lock_recover()`, qui récupère les données même si le mutex a été
  empoisonné par une panique précédente — sans cela, UN incident
  isolé aurait rendu l'API entièrement indisponible en cascade (déni
  de service auto-infligé). Voir le commentaire détaillé dans
  `state.rs`.
- Tous les tris de nombres à virgule flottante utilisent `f64::total_cmp`
  plutôt que `partial_cmp(...).unwrap()`, qui aurait pu paniquer si
  une valeur `NaN` apparaissait un jour dans les données (impossible
  aujourd'hui via l'API JSON, qui rejette `NaN` à la désérialisation,
  mais une protection défensive peu coûteuse).
- `cargo clippy` est exécuté avec `-D warnings` en continu (voir
  `docs/DEVELOPMENT_WORKFLOW.md`) ; les lints stricts
  `clippy::unwrap_used` et `clippy::indexing_slicing` ont été exécutés
  ponctuellement sur tout le code de production (hors tests) : les
  deux seuls cas restants (`median`, `quartiles` dans
  `agoravote-stats`) sont mathématiquement prouvés sûrs et documentés
  comme tels avec `#[allow(...)]` explicite plutôt que masqués
  silencieusement.

## 4. Ce qui n'est PAS couvert (limites connues, à traiter avant toute mise en production)

- **Authentification et autorisation implémentées** (§13, cf. section
  7 ci-dessous) mais avec des permissions encore grossières : un rôle
  `Organizer` administre TOUTES les campagnes de son organisation, pas
  seulement les siennes ; l'inscription publique attribue directement
  ce rôle (pas de flux d'invitation par un administrateur).
- **Pas de limitation de débit (rate limiting)** par IP ou par
  utilisateur : un client peut tenter un nombre illimité de
  connexions (`/auth/login`) ou créer un nombre illimité de comptes en
  boucle — Argon2id ralentit intrinsèquement le brute-force sur un mot
  de passe donné, mais pas les tentatives réparties sur des mots de
  passe différents.
- **Pas de TLS géré par l'application** : le port 3000 sert du HTTP en
  clair. En déploiement réel, un reverse proxy (Traefik, Nginx, Caddy)
  devant le conteneur doit terminer le TLS — non inclus dans le
  `docker-compose.yml` fourni, qui est pensé pour un usage local/test.
- **Pas de validation stricte bulletin ↔ type de question** à la
  soumission (cf. `README.md`, section "ce qui n'est pas fait").
- **Stockage en mémoire par défaut** (si `DATABASE_URL` n'est pas
  définie), non chiffré : le mode PostgreSQL (§12.2) apporte la
  persistance mais pas encore le chiffrement au repos ni la rotation
  des identifiants de connexion — à traiter avant un déploiement
  exposé publiquement (cf. section 6 ci-dessous pour le détail de ce
  qui a changé avec l'introduction de `agoravote-store`).

## 5. Comment vérifier vous-même

```bash
make check                 # fmt + clippy strict + tous les tests
cargo clippy --workspace --all-targets -- -D warnings -W clippy::unwrap_used -W clippy::indexing_slicing
# → doit être silencieux sur le code de production (hors tests, où
#   unwrap()/expect() sont acceptés pour des raisons de lisibilité)
```

## 6. Persistance PostgreSQL — ce qui a changé et pourquoi c'est sûr

L'introduction du crate `agoravote-store` (backend PostgreSQL réel,
cf. `docs/ARCHITECTURE.md`) a été vérifiée contre une vraie base :

- **Mots de passe** : le `docker-compose.yml` fourni utilise un mot de
  passe par défaut (`agoravote`) pour un usage local/test uniquement,
  surchageable via un fichier `.env` (`POSTGRES_PASSWORD=...`) —
  **à changer avant tout déploiement au-delà de votre machine**.
- **Connexion sans TLS par défaut** : le `docker-compose.yml` connecte
  `api` à `db` via le réseau interne Docker (non exposé à l'extérieur
  du host), ce qui rend le TLS entre les deux conteneurs superflu pour
  cet usage. Si `db` devient un service PostgreSQL distant (hors du
  réseau Docker), configurez `DATABASE_URL` avec `?sslmode=require`.
- **Injection SQL** : toutes les requêtes de `agoravote-store` sont
  paramétrées (`sqlx::query(...).bind(...)`), jamais construites par
  concaténation de chaînes — vérifiable directement dans
  `crates/agoravote-store/src/postgres.rs`.
- **Verrouillage transactionnel** : `update_campaign` utilise `SELECT
  ... FOR UPDATE` pour éviter les pertes de mise à jour en cas de
  requêtes concurrentes sur la même campagne (ex : deux tentatives de
  publication simultanées) — testé dans
  `tests/postgres_integration.rs`.
- **Épinglages de dépendances** (`Cargo.toml` de `agoravote-store`) :
  `sqlx` et plusieurs de ses dépendances transitives sont épinglés à
  des versions précises pour des raisons de compatibilité de
  toolchain de développement (documenté ligne par ligne dans le
  fichier), pas pour des raisons de sécurité — la revue de
  vulnérabilités de la section 1 reste valable pour ces versions.

## 7. Authentification (§13) — choix faits et pourquoi

- **Mots de passe** : Argon2id (`argon2` crate), paramètres par
  défaut du crate (recommandés OWASP au moment de l'implémentation).
  Jamais stocké en clair ; format PHC auto-descriptif (algorithme,
  version, coût, sel dans la chaîne elle-même).
- **Jetons de session** : 256 bits d'aléa (`rand::thread_rng`, CSPRNG
  du système), stockés **hachés en SHA-256** en base — jamais en
  clair. Une fuite de la base ne donne donc pas directement de
  sessions utilisables, seulement des empreintes (comme pour les mots
  de passe, mais avec un hachage rapide plutôt que lent : un jeton
  a déjà 256 bits d'entropie aléatoire, contrairement à un mot de
  passe choisi par un humain — pas besoin de ralentir volontairement
  le calcul, cf. commentaire détaillé dans `agoravote-auth/src/session.rs`).
- **Durée de session** : 24h fixes (`SESSION_DURATION`). Pas de
  renouvellement automatique ; une session expirée exige une nouvelle
  connexion.
- **Un jeton invalide n'est jamais traité comme "anonyme" par erreur.**
  Sur la route de vote (authentification optionnelle), l'absence
  totale de jeton est acceptée (vote anonyme, §3.2), mais un jeton
  PRÉSENT-mais-invalide renvoie explicitement `401` plutôt que de
  dégrader silencieusement vers un accès anonyme — cf. la doc de
  `OptionalAuthUser` dans `auth.rs`. Dégrader silencieusement aurait
  été exactement le genre d'"erreur silencieuse" documenté dans
  `docs/LOGGING.md`.
- **Anti-énumération de comptes** : `/auth/login` renvoie le même
  message ("email ou mot de passe incorrect") que l'email soit
  inconnu ou que le mot de passe soit faux — distinguer les deux
  permettrait à un attaquant de découvrir quelles adresses ont un
  compte sans connaître leur mot de passe.
- **`voter_id` ne peut jamais être falsifié par un client authentifié**
  : quand un jeton valide est fourni sur `POST .../ballots`, l'identité
  qu'il porte remplace TOUJOURS un `voter_id` éventuellement présent
  dans le corps de la requête — jamais l'inverse. Vérifié par le test
  `un_participant_authentifie_ne_peut_pas_voter_deux_fois`.
- **Contrainte d'unicité d'email appliquée en base**, pas seulement en
  application : une vérification "cet email existe-t-il déjà ?" suivie
  d'une insertion laisserait une fenêtre de course entre deux
  inscriptions concurrentes avec le même email ; la contrainte `UNIQUE`
  SQL (backend PostgreSQL) et la vérification équivalente du backend
  mémoire ferment cette fenêtre — testé explicitement contre une vraie
  base (`compte_survit_a_un_aller_retour_et_email_est_unique`).

## 8. Identité Ğ1v2 optionnelle (addendum v0.3) — choix faits et limite connue

Cf. `docs/G1_INTEGRATION.md` et `docs/DEVLOG.md` itérations 8-9 pour le
détail de l'implémentation (`POST /auth/g1/challenge`,
`POST /auth/g1/verify`).

- **⚠️ Schéma de signature : ed25519, corrigé en itération 9 — à
  reconfirmer avant mise en production.** Une version antérieure
  vérifiait des signatures sr25519, sur la base d'une hypothèse jamais
  vérifiée ("c'est le défaut Substrate"). Des recherches web (le forum
  et le dépôt git de Duniter restent bloqués par la liste blanche
  réseau de cet environnement, comme `chain.rs` — seuls des extraits
  de moteur de recherche ont pu être consultés) indiquent que les
  comptes/portefeuilles Ğ1v2 réels (Ğecko, Cesium²) signent en
  **ed25519**, le même schéma que la Ğ1 historique, pour
  l'interopérabilité avec les outils tiers — sr25519 étant spécifique
  à l'écosystème Polkadot sans bénéfice propre pour Duniter. Corrigé
  dans `crates/agoravote-g1/src/signature.rs` (désormais
  `ed25519-zebra`, pas `subxt-signer`) — cf. `docs/DEVLOG.md`
  itération 9 pour le détail complet. Cette correction elle-même
  n'est PAS confirmée contre une source primaire (documentation
  officielle, ou test contre un vrai portefeuille) : la même limite
  d'accès réseau qui empêche de vérifier `chain.rs` empêche aussi de
  confirmer ce point avec certitude depuis cet environnement. Avant
  toute mise en production : reconfirmer ce schéma, idéalement en
  signant un défi réel avec un vrai portefeuille Ğ1v2 et en vérifiant
  que `POST /auth/g1/verify` l'accepte.
- **La phrase de 12 mots (ou toute clé privée) ne transite jamais vers
  AgoraVote.** Le protocole défi/signature (`agoravote-g1::challenge`,
  `signature`) ne reçoit et ne manipule que des clés publiques et des
  signatures, produites localement par le portefeuille de
  l'utilisateur — cf. la doc de `agoravote_g1::signature`, qui répète
  cette invariante.
- **Défi à usage unique, avec fenêtre de validité courte (5 minutes)
  vérifiée sans état serveur.** L'horodatage d'expiration est embarqué
  dans le texte du défi lui-même (marqueur `EXP:<epoch>`), donc couvert
  par la signature qu'il produit : un client ne peut pas prolonger la
  validité d'un défi capturé en falsifiant cet horodatage, puisque la
  falsification romprait la correspondance avec la signature déjà
  produite sur le texte d'origine (cf. le test
  `verify_freshness_refuse_un_horodatage_falsifie_a_lavenir` dans
  `crates/agoravote-g1/src/challenge.rs`, qui documente explicitement
  cet invariant). `Challenge::verify_freshness` n'est JAMAIS appelée
  seule côté serveur : toujours en plus de `verify_signature` sur le
  message réellement reçu, jamais à sa place.
- **Message d'erreur générique** (`unauthorized_g1`, même principe que
  `unauthorized_login` ci-dessus) : signature invalide et clé déjà
  liée à un autre compte renvoient la même réponse, pour ne pas
  confirmer à un attaquant qu'une clé publique donnée est déjà
  enregistrée sur la plateforme.
- **Ce que cette connexion NE prouve PAS : l'appartenance à la toile de
  confiance Ğ1.** `agoravote-g1` expose une fonctionnalité séparée,
  `chain::check_membership` (feature `chain-query`), qui interrogerait
  un nœud Ğ1v2 réel pour vérifier qu'un compte est membre actif de la
  toile de confiance — **volontairement non câblée à aucune route de
  ce déploiement**. Deux raisons distinctes, à ne jamais confondre :
  1. **Raison actuelle (limite d'environnement)** : cette vérification
     réseau n'a jamais pu être testée de bout en bout contre un vrai
     nœud `gdev`, l'accès à l'infrastructure Duniter restant bloqué
     dans tous les environnements de développement utilisés jusqu'ici
     (reconfirmé en itération 8) — cf. la règle non négociable de ce
     projet, jamais de code non testé présenté comme fonctionnel.
  2. **Raison durable, même une fois `chain.rs` vérifié** : un nœud RPC
     interrogé pour cette vérification est un tiers de confiance à part
     entière. Un nœud malveillant ou compromis pourrait répondre
     "membre actif" pour un compte qui ne l'est pas (ou l'inverse),
     sans qu'AgoraVote puisse le détecter avec une seule source — cf.
     l'item de `docs/ROADMAP.md` proposant l'interrogation de
     plusieurs nœuds indépendants avant toute décision de légitimité
     de vote qui s'appuierait sur ce résultat. Tant que ce câblage
     n'existe pas, ce risque reste théorique mais doit être traité
     avant toute mise en production s'appuyant sur la toile de
     confiance comme critère d'éligibilité.
  L'API (`routes.rs::g1_verify`) et l'écran frontend `/connexion-g1`
  le disent explicitement à l'utilisateur : cette connexion prouve la
  possession de clé, pas l'appartenance à la toile de confiance.
- **Auto-provisionnement de compte** : la toute première vérification
  réussie pour une clé publique Ğ1 inconnue crée un nouveau `User`
  (rôle `Voter`) sans aucune autre preuve d'identité que la signature
  cryptographique — cohérent avec `/auth/register`, qui ne demande lui
  non plus qu'une adresse email valide. Ce n'est pas une garantie
  d'unicité humaine (rien n'empêche une personne de générer plusieurs
  comptes Ğ1) : seule une vérification de toile de confiance
  (non câblée, cf. ci-dessus) offrirait une garantie plus forte sur ce
  point.
