# Journal de développement

> Les listes "Prochaines itérations candidates" en fin de chaque
> entrée ci-dessous sont des instantanés historiques (ce qui semblait
> prioritaire à ce moment précis) — **pas la source à jour**. Pour les
> priorités actuelles, voir [`docs/ROADMAP.md`](ROADMAP.md).

## Itération 1

Objectif de cette itération : passer du cahier des charges v0.2 à un
premier code qui compile, teste et fait tourner un parcours complet
(créer une campagne → voter → dépouiller), en respectant les
séparations de couches exigées par le document (§4).

## Décisions prises et pourquoi

### 1. Un workspace Cargo à 4 crates, pas un seul binaire

Le cahier des charges (§4) exige que méthode de vote, statistiques et
visualisation restent indépendantes. Une frontière de *module* Rust
(un seul crate avec des `mod`) aurait suffi sur le papier, mais rien
n'empêche un développeur pressé d'importer discrètement un détail
interne d'un autre module dans le même crate. Une frontière de
*crate*, elle, oblige à passer par l'API publique (les traits
exportés) — c'est une garantie structurelle, pas seulement une
convention de code. D'où `agoravote-core` / `agoravote-voting` /
`agoravote-stats` / `agoravote-api`.

### 2. `VotingMethod` comme trait central, pas comme enum

Une alternative plus simple aurait été un `enum VotingMethod { Majority, Approval, Score }`
avec un `match` dans une fonction `tally()`. Rejeté volontairement :
cela recentraliserait toute la logique de calcul dans un seul fichier
et casserait la promesse du §6 ("ajouter, retirer, versionner un
module sans remettre en cause le noyau"). Avec un trait, ajouter une
méthode ne touche à aucun fichier existant sauf le registre (un seul
point d'enregistrement, cf. `docs/ARCHITECTURE.md`).

### 3. `Ballot` porte plusieurs champs optionnels plutôt qu'un enum de bulletins

`selections`, `scores`, `numeric_value`, `text_value` coexistent sur
un seul type `Ballot`, chacun rempli selon le type de question. Un
`enum Ballot { Choice(...), Score(...), ... }` aurait été plus "typé",
mais aurait compliqué le stockage générique et les exports (§14, un
seul format de bulletin à sérialiser) pour un gain de sécurité limité
— la cohérence bulletin/question est de toute façon revérifiée côté
méthode de vote (elle ignore les champs qui ne la concernent pas).
Point de vigilance noté dans `README.md` : cette cohérence n'est pas
encore *validée* à la soumission, seulement tolérée au dépouillement.

### 4. Toolchain rustc 1.75 (apt), pas la dernière stable

L'environnement de développement n'a pas d'accès réseau vers
`static.rust-lang.org` (rustup), seulement vers les dépôts Ubuntu et
crates.io. `apt install rustc cargo` donne rustc 1.75.0. Conséquence
concrète : `uuid` a dû être épinglé à `=1.10.0` (Cargo.toml racine),
car les versions plus récentes tirent une version de `getrandom` qui
exige l'édition Cargo 2024, non supportée par cette toolchain. Ce
n'est pas un choix d'architecture, juste une contrainte d'environnement
— à réévaluer si le projet passe sur une toolchain plus récente
(rustup, ou une image Docker de build dédiée, cf. §12.2).

### 5. `agoravote-api` en mémoire (`Mutex<HashMap<...>>`), pas encore PostgreSQL

Décision de séquencement, pas d'architecture définitive : il fallait
d'abord valider que le modèle §5 et le contrat de module §6
s'assemblent correctement de bout en bout, sans mélanger cette
question avec celle, orthogonale, de la persistance. `state.rs`
documente explicitement ce choix et la façon dont il devra être
défait (remplacer `AppState` par un crate `agoravote-store`, sans
changer les handlers).

## Ce qui a été vérifié concrètement (pas seulement écrit)

- 25 tests unitaires, tous verts (`cargo test --workspace`).
- Parcours HTTP complet exécuté à la main contre le serveur réel :
  création de campagne → formulaire → publication → 6 bulletins →
  dépouillement `voting.majority` avec quorum → relecture du résultat.
  Le calcul (`transport` à 50 %, quorum atteint à 60 % de
  participation sur 10 éligibles) a été vérifié à la main.
- Trois cas d'erreur testés contre le serveur réel : vote après
  clôture (400), publication sans formulaire (400), méthode de vote
  inconnue (400) — aucun des trois ne fait planter le serveur ni ne
  produit un état incohérent.

## Prochaines itérations candidates

Par ordre de dépendance logique (chaque point s'appuie sur les
précédents) :

1. Validation bulletin ↔ type de question à la soumission (actuellement
   seulement tolérée par les méthodes de vote).
2. Journal d'audit (`AuditEvent`) réellement émis par les handlers.
3. Crate `agoravote-store` (PostgreSQL) derrière la même interface que
   `AppState`, pour ne pas avoir à retoucher `routes.rs`.
4. Authentification + RBAC fin (§13) sur les routes d'administration.
5. Première méthode de vote "avancée" (probablement jugement
   majoritaire, plus simple à implémenter et à tester que
   Condorcet/STV) avec confrontation à Condorcet PHP comme oracle de
   test (cf. cahier des charges §2.2, §21).

## Itération 2 — persistance PostgreSQL

Objectif : remplacer le `Mutex<HashMap<...>>` de `agoravote-api` par
une vraie persistance, sans casser l'architecture modulaire ni les
handlers HTTP existants.

### Décisions prises et pourquoi

**Un nouveau crate `agoravote-store`, pas un module dans
`agoravote-api`.** Même raisonnement que pour `agoravote-voting` :
une frontière de crate empêche la persistance de dépendre
accidentellement de détails HTTP, et inversement. Elle ne dépend que
de `agoravote-core` (les entités sérialisables) — ni de
`agoravote-voting`, ni de `agoravote-stats`.

**Un `enum Store { Memory, Postgres }`, pas un trait `dyn Store`.**
Rust 1.75 ne permet pas nativement les fonctions `async` dans un trait
objet sans la dépendance `async-trait` (coût d'allocation par appel).
Avec deux backends seulement, prévus pour ne pas grandir en nombre à
court terme, un `enum` avec un `match` par méthode est plus simple. Le
prix : chaque nouvelle méthode du store doit ajouter une branche dans
les deux implémentations ET dans le `match` de l'enum — un oubli
serait détecté à la compilation (`match` non exhaustif), pas
silencieusement.

**Schéma SQL en JSONB, pas entièrement relationnel.** Toutes les
entités du domaine (`Campaign`, `Form`, `Ballot`, `ResultSet`) sont
déjà sérialisables (`serde`) et exposées telles quelles par l'API HTTP
— dupliquer leur structure en colonnes SQL séparées aurait demandé une
couche de mapping supplémentaire pour un bénéfice nul au stade actuel.
Contrepartie assumée et documentée dans la migration SQL : pas de
contraintes d'intégrité référentielle (clé étrangère) entre les
tables, seulement des index pour les requêtes réellement utilisées.

**Un vrai PostgreSQL local pour tester, pas seulement une relecture de
code.** `postgresql-16` a été installé dans l'environnement de
développement spécifiquement pour valider le crate par des tests
d'intégration réels (`tests/postgres_integration.rs`, 6 tests), plutôt
que de livrer du code SQL jamais exécuté. Un bug aurait été probable
sans ça : la syntaxe `ON CONFLICT (id) DO UPDATE` et le comportement
exact de `FOR UPDATE` dans une transaction ne sont pas des détails
qu'on devine correctement du premier coup.

### Un vrai problème d'environnement, résolu et documenté

La toolchain `rustc 1.75` (la plus récente disponible via `apt` dans
cet environnement) s'est révélée incompatible avec les dépendances
transitives *actuelles* de `sqlx` publiées sur crates.io à la date de
ce travail : plusieurs d'entre elles (`base64ct`, `idna_adapter`,
`native-tls`, `openssl`, `unicode-segmentation`, `idna`, `crc`) ont,
dans leurs dernières versions, relevé leur exigence de compilateur
au-delà de 1.75, ou requièrent l'édition Cargo 2024. Diagnostic posé
par bisection (reproduction isolée hors du workspace, dans
`/tmp/sqlx-repro`, en ajoutant les dépendances une par une jusqu'à
obtenir une compilation propre) plutôt que par supposition. Résultat :
`sqlx` redescendu en version 0.6 et sept dépendances transitives
épinglées à des versions précises, chacune commentée individuellement
dans `crates/agoravote-store/Cargo.toml` avec la raison exacte de son
épinglage. Ces pins n'ont aucune conséquence sur l'image Docker de
production, qui utilise une toolchain à jour — ils ne concernent que
la reproductibilité du développement dans CET environnement, et
seront à réévaluer si la toolchain locale est mise à jour.

### Ce qui a été vérifié concrètement

- 6 tests d'intégration contre un vrai PostgreSQL 16 (`cargo test -p
  agoravote-store -- --ignored`), tous verts.
- 29 tests du reste du workspace toujours verts après le câblage
  (`cargo test --workspace`), `cargo fmt --check` et
  `clippy -D warnings` toujours silencieux.
- **Test de survie réel** : serveur démarré avec `DATABASE_URL`,
  campagne + formulaire + 3 bulletins + dépouillement créés via l'API,
  **processus arrêté complètement** (`pkill`), données relues
  directement en base (hors API) pour confirmer leur présence,
  **serveur redémarré**, mêmes données relues cette fois via l'API —
  identiques au bulletin près. C'est la preuve qui manquait à la
  version précédente ("tout disparaît au redémarrage").

### Prochaines itérations candidates (mise à jour)

1. Authentification + RBAC fin (§13) — devient prioritaire maintenant
   que les données persistent réellement.
2. Validation bulletin ↔ type de question à la soumission.
3. Journal d'audit (`AuditEvent`) réellement émis par les handlers,
   lui-même persisté via `agoravote-store`.
4. Module d'identité Ğ1/Duniter (cf. addendum v0.3) — étude de
   faisabilité en lecture seule.
5. Première méthode de vote "avancée" (jugement majoritaire).

## Itération 3 — authentification

Objectif : donner un vrai sens à la persistance de l'itération 2 en
empêchant n'importe qui de créer/publier/clôturer une campagne, et en
apportant l'intégrité minimale attendue d'un scrutin (un participant
identifié ne vote qu'une fois).

### Décisions prises et pourquoi

**Un nouveau crate `agoravote-auth`, séparé de `agoravote-store`.**
Même logique que pour `agoravote-voting` : la LOGIQUE de hachage et de
génération de jeton (pure, sans I/O) est séparée de la PERSISTANCE de
ces informations. `agoravote-store` gagne juste de nouvelles tables
(`users`, `accounts`, `sessions`) et de nouvelles méthodes, sans avoir
à connaître Argon2 ni la génération d'aléa.

**Jeton opaque stocké haché, pas de JWT.** Un JWT est auto-porteur
(l'identité et les rôles sont dans le jeton signé) mais pas révocable
avant expiration sans liste de révocation — qui réintroduit de toute
façon un état côté serveur. Autant garder un jeton opaque dès le
départ. Et comme pour un mot de passe : si la base fuit, un attaquant
ne doit pas repartir avec des sessions utilisables — d'où le stockage
haché (SHA-256, rapide, puisque contrairement à un mot de passe le
jeton a déjà 256 bits d'aléa).

**`voter_id` du client jamais fait confiance quand un jeton est
présent.** Avant cette itération, `cast_ballot` acceptait tel quel le
`voter_id` fourni dans le corps de la requête — n'importe qui pouvait
prétendre être n'importe quel votant. Avec l'authentification, un
jeton valide écrase systématiquement ce champ. C'est aussi ce qui rend
l'anti double-vote possible : sans identité fiable, "empêcher un
second vote" n'a pas de sens à faire respecter.

**Authentification optionnelle sur le vote, obligatoire sur
l'administration.** Cohérent avec §3.2 (un sondage peut être public,
sans compte) et avec le principe déjà établi de séparation
identité/bulletin (§13) : un scrutin anonyme reste possible, un
scrutin nominatif gagne l'intégrité en plus.

### Un vrai problème d'environnement, résolu plus vite cette fois

Même famille de problème que pour `sqlx` (itération 2) : `argon2` /
`password-hash` dépendent de `base64ct`, dont les dernières versions
exigent l'édition Cargo 2024. Ayant déjà la méthode (bisection, pins
documentés), le correctif a pris une commande au lieu d'une session
de débogage complète : `base64ct = "=1.6.0"`, `rand = "=0.8.5"`,
`rand_core = "=0.6.4"` appliqués préventivement dans
`agoravote-auth/Cargo.toml`, compilation réussie du premier coup.

### Ce qui a été vérifié concrètement

- 8 tests unitaires `agoravote-auth` (hachage/vérification de mot de
  passe, unicité du sel, génération/expiration de session).
- 9 tests d'intégration `agoravote-api` (dont : création de campagne
  sans jeton → 401, mauvais mot de passe → 401, double inscription
  même email → refusée, double vote d'un participant identifié →
  refusé).
- 8 tests d'intégration `agoravote-store` contre PostgreSQL réel
  (dont les nouveaux : compte + contrainte d'unicité, cycle de vie
  d'une session).
- **Parcours complet contre le serveur réel** (PostgreSQL, pas la
  mémoire) : campagne refusée sans jeton → inscription → campagne
  acceptée avec jeton → connexion avec casse d'email différente →
  mauvais mot de passe refusé → double inscription refusée → premier
  vote de Bob accepté → second vote de Bob refusé → vote anonyme
  toujours accepté. Les neuf étapes ont produit exactement le
  comportement attendu.
- `cargo fmt --check` et `clippy -D warnings` silencieux sur
  l'ensemble du workspace après ajout du crate.

### Prochaines itérations candidates (mise à jour)

1. Permissions fines (une campagne appartient à un organisateur
   précis, pas à "tout organisateur de l'organisation").
2. Flux d'invitation d'organisateurs (au lieu d'une inscription
   publique attribuant directement ce rôle).
3. Validation bulletin ↔ type de question à la soumission.
4. Journal d'audit (`AuditEvent`) réellement émis et persisté.
5. Module d'identité Ğ1/Duniter (cf. addendum v0.3).
6. Première méthode de vote "avancée" (jugement majoritaire).

## Itération 4 — module d'identité Ğ1v2 (exploratoire)

Objectif : évaluer la faisabilité d'une identité optionnelle adossée à
la toile de confiance de la Ğ1v2 (cf. addendum v0.3), puis commencer
l'implémentation. Contrairement aux itérations précédentes, cette
itération n'a **pas** pu être entièrement vérifiée — documenté
honnêtement plutôt que masqué.

### Recherche avant code

Avant d'écrire une ligne, vérification que la Ğ1v2 (et non la v1,
protocole totalement différent) était bien identifiée : migration en
production le 7 mars 2026, pallets confirmés par lecture directe de la
documentation générée du runtime (`Identity`, `Certification`,
`Distance`, `UniversalDividend`). Testé aussi, concrètement, l'accès
réseau depuis cet environnement vers l'infrastructure Duniter — bloqué
par la liste blanche réseau du bac à sable (`Host not in allowlist`),
contrairement à PostgreSQL qui avait pu être installé localement.

### Un incident révélateur, corrigé

Le crate `agoravote-g1` a été ajouté au workspace principal avant que
sa compilabilité ne soit établie — et a cassé `cargo build --workspace`
pour tout le projet (dépendances cryptographiques transitives exigeant
un rustc plus récent). Corrigé en sortant ce crate du workspace
(section `[workspace]` vide dans son propre `Cargo.toml`, son propre
`Cargo.lock`), avec des valeurs de package explicites plutôt
qu'héritées. Les 44 tests du workspace principal ont été revérifiés
verts après coup. C'est exactement le genre d'erreur que
`docs/DEVELOPMENT_WORKFLOW.md` (`make check` avant tout commit) est
censé prévenir — leçon pour la suite : lancer `make check` après
l'ajout de tout nouveau crate, même expérimental.

### Bisection de dépendances : plus loin que jamais, mais bloquée

Même méthode que pour `sqlx` (itération 2), poussée sur un arbre bien
plus vaste (cryptographie Substrate complète) : neuf épinglages
successifs (`constant_time_eq`, `indexmap`, `zeroize`/`zeroize_derive`,
`toml_parser`, `parity-scale-codec`, `parity-scale-codec-derive`) ont
chacun fait progresser la résolution d'un cran, avant un blocage final
sur `toml_datetime` exigeant rustc 1.76+, réclamé par une dépendance
qu'il n'a pas été possible d'identifier sans résolution complète
(`cargo tree` nécessite lui-même une résolution réussie). Détail
complet, avec chaque commande exacte essayée, dans le `Cargo.toml` du
crate — pour que quiconque reprend cette piste avec une toolchain à
jour n'ait pas à repartir de zéro.

### Un correctif de rigueur pendant la rédaction de la doc

En documentant "`cargo test -p agoravote-g1 --lib challenge`
fonctionne", vérification que c'était *littéralement* vrai avec le
`Cargo.toml` réel du crate — ce ne l'était pas : les dépendances
cryptographiques n'étaient pas encore optionnelles, donc cette
commande aurait échoué. Corrigé en restructurant le crate en features
(`signature-verification`, `chain-query`, toutes deux désactivées par
défaut) pour que `challenge.rs` compile et teste réellement, seul, par
défaut. Petite chose, mais révélatrice : mieux vaut tester une
affirmation de documentation avant de l'écrire que la corriger après
coup — leçon déjà tirée plusieurs fois dans ce projet (cf. itération 1,
le bug d'ordre des middlewares détecté par un test).

### Ce qui a été vérifié concrètement

- 3 tests unitaires `agoravote-g1` (génération/expiration de défi),
  tous verts — la seule partie du module vérifiable sans dépendance
  cryptographique ni accès réseau.
- Confirmation que `cargo build --workspace`/`cargo test --workspace`
  (44 tests) et `clippy -D warnings` restent intacts après
  l'introduction de ce crate expérimental.
- Accès réseau à l'infrastructure Ğ1v2 testé et confirmé bloqué depuis
  cet environnement (preuve par `curl`, pas supposition).

### Ce qui reste non vérifié (honnêteté du statut)

`signature.rs` (vérification sr25519) et `chain.rs` (requête de statut
de membre) sont écrits avec soin, à partir de sources vérifiées, mais
n'ont pu être ni compilés ni testés ici. Marche à suivre documentée
dans `crates/agoravote-g1/README.md` et `docs/G1_INTEGRATION.md`.

### Prochaines itérations candidates (mise à jour)

1. Compiler/vérifier `agoravote-g1` dans un environnement à toolchain
   à jour, puis le réintégrer au workspace principal.
2. Permissions fines (une campagne appartient à un organisateur
   précis, pas à "tout organisateur de l'organisation").
3. Flux d'invitation d'organisateurs.
4. Validation bulletin ↔ type de question à la soumission.
5. Journal d'audit (`AuditEvent`) réellement émis et persisté.
6. Première méthode de vote "avancée" (jugement majoritaire).

## Itération 5 — frontend React et contrat d'API

Objectif : construire une interface web consommant l'API, en
choisissant délibérément de figer le contrat d'API en premier
(OpenAPI) plutôt que de commencer directement par l'UI.

### Décisions prises et pourquoi

**OpenAPI comme source de vérité, types TypeScript générés.** Plutôt
que d'écrire à la main des interfaces TypeScript dupliquant les
structures Rust (avec le risque classique de désynchronisation
silencieuse au fil du temps), `docs/openapi.yaml` documente l'API
telle qu'elle existe réellement, et `npm run gen:api`
(`openapi-typescript`) en dérive les types consommés par le frontend.
Un changement d'API non répercuté dans la spec se traduit par une
erreur de compilation TypeScript, pas par un bug silencieux à
l'exécution — même philosophie que partout ailleurs dans ce projet.

**Tous les champs de réponse marqués `required` dans la spec.** Une
première version de la spec ne le faisait pas, ce qui a fait
apparaître des dizaines d'erreurs "possibly undefined" en cascade dans
le frontend — corrigé à la source (la spec) plutôt qu'en ajoutant des
`!`/vérifications défensives partout dans le code consommateur, qui
auraient masqué le vrai problème (la spec décrivait l'API en dessous
de ses garanties réelles).

**Un vrai manque d'API découvert en construisant le frontend :**
`GET /campaigns/:id/form` n'existait pas — seulement sa création. Sans
cette route, impossible d'afficher une question à un citoyen avant de
voter. Ajoutée, testée (2 nouveaux tests d'intégration), documentée.
C'est un exemple concret de la valeur de construire le frontend et le
backend en dialogue plutôt que le frontend en aval, une fois le
backend "figé".

**"Campagnes récentes" mémorisées côté navigateur, pas une vraie liste
serveur.** L'API n'expose aucune route `GET /campaigns` (liste) — un
manque identifié mais volontairement non comblé cette fois (contexte
de temps), documenté comme palliatif assumé plutôt que caché.

**Design délibéré plutôt que le défaut générique.** Palette "papier de
scrutin civique" (encre, gris-vert chaud, accent vert civique unique)
choisie explicitement contre les deux dérives les plus fréquentes
d'une interface générée : le combo crème/terracotta et le kit SaaS de
cartes identiques — cf. `frontend/DESIGN.md` pour le plan complet et
sa revue contre ces deux écueils.

### Ce qui a été vérifié concrètement

- `npx tsc -b` (compilation TypeScript stricte) : zéro erreur.
- `npx oxlint` : zéro erreur, 4 avertissements mineurs (motifs React
  standards de fetch de données en effet).
- `npm run build` : build de production réussi, polices auto-hébergées
  confirmées dans le bundle final (pas de CDN externe).
- 2 nouveaux tests d'intégration backend pour `GET .../form` (public,
  et 404 si absent) — 46 tests au total sur le workspace, toujours
  verts après cette itération.
- `docker-compose.yml`, `frontend/Dockerfile` et `nginx.conf` validés
  syntaxiquement (YAML, structure Dockerfile) mais **jamais construits
  ni exécutés réellement** — aucun démon Docker disponible dans cet
  environnement (même limite déjà documentée pour le backend).

### Ce qui reste non vérifié (honnêteté du statut)

**Aucune vérification visuelle n'a été possible.** Ce frontend n'a
jamais été ouvert dans un navigateur réel — la compilation et le build
réussissent, ce qui élimine une classe entière de bugs (erreurs de
type, imports cassés, syntaxe), mais ne garantit rien sur le rendu
visuel, la mise en page réelle, ou des bugs d'interaction qu'un test
manuel révélerait immédiatement. Premier test à faire par
l'utilisateur : `docker compose up` puis `http://localhost:8080`.

### Prochaines itérations candidates (mise à jour)

1. Ouvrir le frontend dans un vrai navigateur, corriger ce qui ne va
   pas visuellement (quasi certain qu'il y aura quelque chose).
2. `GET /campaigns` (liste) côté API, pour remplacer le palliatif de
   mémorisation locale du tableau de bord.
3. Compiler/vérifier `agoravote-g1` dans un environnement à toolchain
   à jour, puis le réintégrer au workspace principal.
4. Permissions fines, flux d'invitation d'organisateurs, journal
   d'audit (cf. itération 3).
5. Types de question supplémentaires dans le constructeur de
   formulaire (texte, nombre, échelle, classement).

## Itération 6 — nouvel environnement : vérification visuelle, `agoravote-g1`, épinglages

Objectif : ce projet change d'environnement de développement pour la
première fois (cf. `CLAUDE.md`, section "Contexte d'origine
important") — vérifier ce qui devient possible, et traiter en priorité
les deux items de tête de `docs/ROADMAP.md` ("Priorité immédiate") qui
en dépendaient : vérification visuelle du frontend, et compilation de
`agoravote-g1`.

### Ce qui a changé d'environnement (constaté, pas supposé)

- `rustc 1.94.1` / `cargo 1.94.1` (contre 1.75 via apt auparavant).
- `docker 29.3.1` et `docker compose v5.1.1` installés, démon
  fonctionnel après démarrage manuel (`dockerd` en tâche de fond — le
  script `/etc/init.d/docker` échoue sur un `ulimit` sans rapport).
  **Mais** les couches d'images Docker Hub (`production.cloudfront.docker.com`)
  restent bloquées par la politique réseau de ce bac à sable (`403`
  côté proxy sortant) : `docker compose up` reste donc inutilisable
  ici, contrairement à ce qu'annonçait `CLAUDE.md`. PostgreSQL 16 et
  Node 22/npm sont en revanche installés nativement et pleinement
  utilisables.
- Accès réseau à `crates.io`/`static.rust-lang.org` confirmé (`200`).
  Accès à l'infrastructure Duniter/Ğ1 (`rpc.duniter.org`,
  `g1-squid.axiom-team.fr`, tout nœud `gdev`) **toujours bloqué**
  (`403` côté proxy sortant) — testé explicitement, pas supposé
  inchangé.
- Chromium pré-installé (`/opt/pw-browsers`), utilisable avec
  Playwright pour piloter un vrai navigateur — première fois que ce
  projet peut vérifier visuellement son frontend.

### `agoravote-g1` : compilé et testé pour la première fois

`cargo build -p agoravote-g1 --features chain-query` échouait
auparavant faute de toolchain compatible (cf. itération 4). Avec
rustc 1.94, il compile **intégralement**, `chain-query` compris. Une
seule correction d'API a été nécessaire (l'hypothèse documentée dans
`signature.rs` était fausse) : `verify` est une fonction libre du
module `subxt_signer::sr25519`, pas une méthode de `PublicKey`.

Une seconde correction, découverte en écrivant le test manquant
(ci-dessous) : la feature `sr25519` de `subxt-signer` seule ne suffit
pas à signer (uniquement à vérifier) — sans la feature `std`, la
génération d'aléa (`schnorrkel` → `getrandom_or_panic`) panique
volontairement plutôt que d'utiliser un RNG non cryptographique.
Ajoutée aux features activées par `signature-verification`.

`challenge.rs` et `signature.rs` sont désormais compilés **et
testés** au même niveau d'exigence que le reste du projet (9 tests
verts, `cargo test -p agoravote-g1 --features chain-query`) : les 2
tests manquants identifiés dans le README du crate ont été ajoutés,
contre le compte de développement Substrate standard `//Alice`
(vecteur sr25519 connu, jamais un vrai compte Ğ1) — signature valide
acceptée, signature valide mais message modifié refusée.

`chain.rs` compile également mais reste **non vérifié à l'exécution** :
l'infrastructure réseau Duniter/Ğ1 est toujours bloquée dans cet
environnement (voir ci-dessus), donc les noms de stockage
(`IdentityIndexOf`, `Membership`) restent des hypothèses non
confirmées contre un nœud réel. Le crate reste donc volontairement
hors du workspace principal (cf. `crates/agoravote-g1/README.md`,
section "Pour le rendre entièrement utilisable" mise à jour) —
seule l'étape 2 de son propre plan (confirmer les noms de stockage)
reste bloquée, pas techniquement mais par l'accès réseau.

### Réévaluation des épinglages de version

Avec rustc 1.94 (bien au-delà des seuils 1.76/1.79/1.80/1.81/1.83/1.85
qui avaient motivé chaque épinglage) :

- `uuid` (racine, `[workspace.dependencies]`) : épinglage `=1.10.0`
  retiré, remonté à `1.26.1` sans changement de code nécessaire.
- `agoravote-store` : les 7 épinglages transitifs de `sqlx`
  (`idna_adapter`, `native-tls`, `openssl`/`openssl-sys`,
  `unicode-segmentation`, `idna`, `url`, `crc`) retirés — `cargo
  build`/`cargo test` résolvent désormais des versions plus récentes
  sans intervention. Revérifié contre un vrai PostgreSQL 16 local (8
  tests d'intégration, tous verts) après ce changement, pas seulement
  compilé.
- `sqlx` lui-même **reste en 0.6** (pas de migration vers 0.7/0.8) :
  la contrainte de toolchain qui l'imposait est levée, mais une
  migration de version majeure changerait l'API macros/offline/
  executor dans tout `agoravote-store` (déjà testé contre PostgreSQL
  réel) pour un gain non demandé par cette itération — proposé comme
  item de roadmap séparé plutôt que fait à la volée.

### Frontend vérifié dans un vrai navigateur — un bug réel trouvé et corrigé

Premier `npm install` réel de ce projet : a immédiatement échoué
(`ERESOLVE`) — `typescript` épinglé sur `~6.0.2` dans `package.json`
est incompatible avec le `peerDependencies` de `openapi-typescript`
(`^5.x`, toujours vrai en version 7.13.0, la plus récente). Ce
conflit existait déjà dans le `package-lock.json` commité, jamais
détecté faute d'avoir pu lancer un `npm install` propre auparavant.
Corrigé : `typescript` ramené à `^5.9.3` (dernière 5.x stable),
`package-lock.json` régénéré. `npx tsc -b`, `npx oxlint` et `npm run
build` toujours verts après ce changement.

Backend lancé en local (`cargo run -p agoravote-api`, PostgreSQL réel
— `docker compose` indisponible, cf. ci-dessus) et frontend en mode
développement (`npm run dev`, proxy Vite déjà configuré vers le port
3000), pilotés avec Chromium/Playwright : parcours complet conduit de
bout en bout — inscription, création de campagne, construction du
formulaire, publication, vote citoyen (session anonyme), dépouillement
`voting.approval`, page de résultats publique — plus la page de
connexion et la page d'accueil/inscription en viewport mobile
(390×844). Rendu visuel conforme à la direction "papier de scrutin
civique" documentée dans `frontend/DESIGN.md`, mise en page mobile
sans débordement, zéro erreur console sur l'ensemble du parcours.

**Un bug réel trouvé** : après un rechargement complet de page sur une
route admin (`/admin`, `/admin/campagnes/:id`), l'en-tête n'affichait
plus AUCUN lien (ni "Tableau de bord/Se déconnecter", ni "Se
connecter/Créer un compte") et "Bonjour {nom}" s'affichait vide —
alors que le jeton de session restait valide en `localStorage`, donc
l'utilisateur restait bel et bien connecté côté serveur. Cause : `Shell.tsx`
conditionne l'affichage sur `status === "authenticated" && user`, mais
`AuthContext` ne remplissait `user` qu'au moment d'un `login`/
`register` — jamais revalidé au chargement de page à partir d'un jeton
déjà stocké, faute de route "qui suis-je" côté API (limite déjà
documentée dans le code, `AuthContext.tsx`, avant cette itération).
Pire : `AdminHome.handleCreate` fait `if (!user) return;` — "Créer la
campagne" échouait alors silencieusement après un rechargement, sans
aucun message d'erreur. `App.tsx` anticipait même ce problème dans un
commentaire (`status === "checking"`) sans que l'état correspondant
n'ait jamais été implémenté.

Corrigé de bout en bout, suivant la convention du projet
(`docs/openapi.yaml` source de vérité en premier) :
1. `GET /auth/me` ajouté à `docs/openapi.yaml`, implémenté dans
   `agoravote-api/src/routes.rs` (réutilise l'extracteur `AuthUser`
   déjà validé ailleurs, aucune logique nouvelle), 2 nouveaux tests
   d'intégration (jeton valide → utilisateur renvoyé ; sans jeton →
   401).
2. Types frontend régénérés (`npm run gen:api`).
3. `AuthContext.tsx` : ajout d'un état `"checking"` (que `App.tsx`
   attendait déjà) entre le montage et la confirmation du jeton par
   `GET /auth/me` ; en cas d'échec (jeton expiré/invalide), la session
   locale est effacée plutôt que de rester bloquée en
   "authenticated" sans utilisateur.
4. `App.tsx` (`RequireAuth`) et `Home.tsx` mis à jour pour traiter
   explicitement ce nouvel état plutôt que de tomber implicitement
   dans la branche "anonyme".

Revérifié après correction, dans les mêmes conditions (vrai navigateur,
rechargement complet de `/admin`) : en-tête et tableau de bord
corrects, zéro erreur console.

### Ce qui a été vérifié concrètement

- `agoravote-g1` : 9 tests verts (`cargo test -p agoravote-g1
  --features chain-query`), `cargo fmt --check` et
  `clippy --all-features -- -D warnings` silencieux.
- Workspace principal : `make check` vert (fmt, clippy, 46 tests) à la
  fois avant et après le retrait des épinglages de version, plus 8
  tests d'intégration PostgreSQL réels revérifiés séparément après ce
  retrait.
- Deux nouveaux lints clippy apparus avec rustc 1.94
  (`manual_is_multiple_of`) corrigés dans `agoravote-stats` (préexistant
  au reste de cette itération, découvert par le premier `make check`
  lancé dans ce nouvel environnement).
- Frontend : `npx tsc -b`, `npx oxlint`, `npm run build` verts ; parcours
  complet vérifié dans un vrai Chromium (desktop et mobile), avant et
  après le correctif `/auth/me`.
- `.gitignore` ajouté à la racine (absent jusqu'ici — `target/` et
  `node_modules/` n'étaient tout simplement jamais apparus dans aucun
  environnement de développement précédent, faute d'avoir pu réellement
  compiler/installer).

### Ce qui reste non vérifié (honnêteté du statut)

`chain.rs` (`agoravote-g1`) compile mais reste non exécuté contre un
nœud Ğ1v2 réel — accès réseau à cette infrastructure toujours bloqué
dans tous les environnements de développement utilisés jusqu'ici. Les
noms de stockage qu'il interroge restent des hypothèses.

### Prochaines itérations candidates (mise à jour)

1. Confirmer les noms de stockage de `chain.rs` contre un nœud `gdev`
   réel, depuis un environnement qui a accès à l'infrastructure
   Duniter — puis réintégrer `agoravote-g1` au workspace principal.
2. `GET /campaigns` (liste) côté API, pour remplacer le palliatif de
   mémorisation locale du tableau de bord.
3. Migration `sqlx` 0.6 → 0.7/0.8 (désormais possible techniquement,
   pas faite cette itération — cf. ci-dessus).
4. Permissions fines, flux d'invitation d'organisateurs, journal
   d'audit (cf. itération 3).
5. Types de question supplémentaires dans le constructeur de
   formulaire (texte, nombre, échelle, classement).
6. `cargo audit` en CI (désormais possible avec rustc 1.94, cf.
   `docs/SECURITY.md` §1) — pas fait cette itération.

## Itération 7 — refonte frontend sur les planches UX fournies

Objectif : le porteur de projet a fourni les six planches d'interface
référencées au §10 du cahier des charges (jusqu'ici jamais vues) ainsi
que le cahier des charges v0.2 et l'addendum v0.3 eux-mêmes (ni l'un ni
l'autre n'étaient committés dans ce dépôt malgré les renvois du README
et de `CLAUDE.md` — même lacune que `CLAUDE.md` avant l'itération 6,
corrigée de la même façon). Objectif de cette itération : aligner le
frontend sur ces planches, jamais vues lors de sa conception initiale
(cf. itération 5, direction "papier de scrutin civique" inventée sans
y avoir accès).

### Décision : remplacer la direction visuelle, pas la retoucher

`frontend/DESIGN.md` prévoyait déjà une nav latérale pour l'admin
("cohérent avec les six planches de maquette fournies au tout début du
projet") mais ne l'avait jamais construite — l'écart entre l'intention
documentée et le code livré n'avait jamais été remarqué faute d'avoir
pu ouvrir un navigateur avant l'itération 6. Les planches montrent une
direction éloignée du vert civique choisi précédemment (accent bleu,
cartes statistiques colorées, fond très clair, typographie sans-serif
partout, pas de serif éditoriale) — remplacée intégralement plutôt que
mélangée avec l'ancienne, pour rester cohérente. `@fontsource/fraunces`
retiré (plus aucun usage).

### Palette et graphiques : passés par la skill dataviz du projet

Avant d'écrire le moindre graphique (jauge de participation, barres,
donut), la palette catégorielle et les règles de marque (barres
arrondies 4px, espace de 2px entre segments, légende obligatoire à
partir de 2 séries, jamais de couleur recyclée par index) ont été
prises de la skill `dataviz` disponible dans cet environnement plutôt
qu'improvisées — la palette catégorielle a été revalidée avec son
script (`validate_palette.js`, tous les seuils CVD/contraste passent).

### Nav latérale — honnête sur ce qui existe vraiment

`AdminLayout` (nouveau) affiche les six entrées des planches
(Tableau de bord, Campagnes, Modules, Utilisateurs, Résultats,
Paramètres) mais seules trois pointent vers une vraie route : les
trois autres n'ont aucun backend derrière (pas de gestion
d'utilisateurs, pas de paramètres d'organisation, pas d'agrégation
"tous les résultats") et sont affichées grisées avec un badge
"Bientôt" plutôt qu'omises (fidélité visuelle aux planches) ou
cliquables vers du vide (jamais de fonctionnalité fictive).

Deux nouvelles pages sont réellement branchées à des données existantes
plutôt qu'inventées :
- **Campagnes** (`/admin/campagnes`) : reprend la même limite déjà
  assumée par `lib/recentCampaigns.ts` (liste propre à ce navigateur,
  pas une vraie liste serveur — `GET /campaigns` n'existe toujours pas
  côté API, cf. roadmap).
- **Modules** (`/admin/modules`) : consomme `GET /modules`, déjà réel
  et déjà utilisé en interne par le sélecteur de méthode, mais jamais
  montré comme écran à part entière (écran "16. Modules" du §10.1,
  jusqu'ici seulement câblé, pas affiché).

### Éditeur de formulaire : les quatre types manquants, pas inventés

`docs/openapi.yaml` acceptait déjà six types de question
(`single_choice`, `multiple_choice`, `text`, `number`, `scale`,
`ranking`) mais seuls les deux premiers avaient une UI (cf. ROADMAP,
"types de question supplémentaires"). Les quatre autres sont ajoutés à
la bibliothèque de questions du nouvel éditeur (bibliothèque + canevas,
planche 2), avec les champs de configuration exacts du DTO Rust
(`agoravote_core::form::QuestionType`, vérifié dans le code plutôt que
supposé) : `max_length` pour `text`, `min`/`max` pour `number`,
`min`/`max` obligatoires pour `scale`, `options` pour `ranking`.
Referme un item de ROADMAP resté ouvert depuis l'itération 5.

### Écran "Analyse" (planche 6) : scope volontairement réduit, jamais de donnée fictive

La planche 6 montre une évolution temporelle de la participation et
des comparaisons par groupe d'âge/territoire. Aucune des deux dimensions
n'existe dans le modèle de données actuel : `Ballot` n'a ni série
temporelle exploitée ni attribut démographique (cf.
`agoravote_core::ballot::Ballot`), et `agoravote-stats` (moyenne,
médiane, écart-type, tableau croisé) est une dépendance déclarée de
`agoravote-api` mais n'est câblée à AUCUNE route — vérifié par
recherche dans le code (`grep` de `descriptive::`/`crosstab::`), pas
supposé. Plutôt que fabriquer ces graphiques avec des données
inventées — contraire à la règle non négociable de ce projet —, l'écran
construit (`/admin/campagnes/:id/analyse`) se limite à ce qui est
réellement mesurable aujourd'hui : comparaison des `ResultSet` déjà
calculés entre les questions d'une même campagne, avec une bascule
graphique/tableau (les deux vues des mêmes données réelles, pas deux
jeux de données différents). Le reste (démographie, séries temporelles,
statistiques descriptives) est noté en limite connue ci-dessous et dans
`docs/ROADMAP.md`, pas simulé.

### Deux bugs réels trouvés en repassant le parcours en navigateur

1. **Nav du haut cassée en mobile pour un utilisateur connecté.** Sur
   l'écran de vote (route publique, nav du haut `Shell.tsx`, pas la nav
   latérale), les trois éléments ("Tableau de bord", nom, "Se
   déconnecter") se chevauchaient avec le logo en dessous de 400px de
   large — jamais remarqué avant faute d'avoir testé cet état précis en
   mobile lors de l'itération 6. Corrigé : le lien "Tableau de bord" et
   le nom affiché se masquent sous le seuil `sm`, ne laissant que le
   bouton de déconnexion (l'action réellement nécessaire depuis cet
   écran).
2. **Nav latérale à largeur fixe cassant tout le contenu en mobile.**
   `AdminLayout` utilisait une colonne `w-64` (256px) fixe à côté du
   contenu — sur un viewport de 390px, il ne restait que 70px pour tout
   le contenu admin (formulaire, cartes, tableaux), rendu totalement
   inutilisable (texte un mot par ligne). Trouvé par inspection directe
   du DOM (`getBoundingClientRect`), pas seulement visuellement. Corrigé
   en tiroir superposé (fermé par défaut, bouton hamburger, recouvrement
   assombri fermable au clic) sous le seuil `lg` ; le rendu desktop
   (nav latérale statique) reste inchangé au-dessus de ce seuil — cf.
   §11 du cahier des charges, "administration desktop-first" n'excuse
   pas un rendu cassé en dessous.

### Ce qui a été vérifié concrètement

- `npx tsc -b`, `npx oxlint`, `npm run build` verts après chaque étape
  significative de la refonte, pas seulement à la fin.
- Parcours complet rejoué dans un vrai Chromium après la refonte :
  inscription, tableau de bord, Campagnes, Modules, création de
  campagne, éditeur de formulaire (avec un type "Échelle" en plus de
  choix unique), configuration du scrutin (cartes de méthode), vote
  citoyen, dépouillement, page de résultats publique (jauge + donut +
  barres + méthodologie), écran Analyse (vue graphique et vue tableau).
  Zéro erreur console sur l'ensemble, en desktop et en mobile (390px),
  y compris le tiroir de nav latérale ouvert/fermé.
- `make check` (workspace Rust, inchangé cette itération) toujours vert
  après la refonte frontend, pour confirmer qu'aucun changement côté
  contrat d'API n'a été introduit par erreur.
- Palette catégorielle revalidée avec le script de la skill dataviz
  (`validate_palette.js`), tous les seuils CVD/contraste "PASS".

### Ce qui reste non vérifié / non fait (honnêteté du statut)

- **Fidélité pixel-perfect** aux planches fournies : la structure, la
  hiérarchie et les couleurs suivent les planches, mais les valeurs
  exactes (espacements, tailles de police précises) n'ont pas été
  mesurées pixel par pixel contre les images fournies — jugement visuel
  humain requis pour une revue plus fine si nécessaire.
- **Écran "Analyse"** : démographie, séries temporelles et statistiques
  descriptives (moyenne/médiane/écart-type) restent hors périmètre,
  faute de modèle de données et de route API — cf. ci-dessus et
  `docs/ROADMAP.md`.
- **`GET /campaigns` (liste serveur)** : toujours absent côté API ;
  l'écran "Campagnes" ajouté cette itération reste un palliatif
  côté navigateur, pas la solution cible.

### Prochaines itérations candidates (mise à jour)

1. Confirmer les noms de stockage de `chain.rs` (`agoravote-g1`) contre
   un nœud `gdev` réel, depuis un environnement qui a accès à
   l'infrastructure Duniter — puis réintégrer au workspace principal.
2. `GET /campaigns` (liste) côté API, pour remplacer le palliatif de
   mémorisation locale (tableau de bord ET nouvel écran Campagnes).
3. Câbler `agoravote-stats` à une route API (statistiques descriptives
   par question) pour enrichir l'écran Analyse sans donnée fictive.
4. Étudier l'ajout d'attributs démographiques optionnels au modèle de
   participation, si le projet veut un jour les comparaisons par
   groupe montrées en planche 6 — implique une décision produit sur la
   collecte de ces données (cf. §13 "minimisation" du cahier des
   charges), pas seulement un ajout technique.
5. Migration `sqlx` 0.6 → 0.7/0.8 (cf. itération 6).
6. Permissions fines, flux d'invitation d'organisateurs, journal
   d'audit (cf. itération 3).

## Itération 8 — identité Ğ1v2 optionnelle câblée de bout en bout

Objectif : le porteur de projet a demandé une analyse d'écart au
cahier des charges, en insistant explicitement sur l'identité Ğ1v2
optionnelle (addendum v0.3) — « attention à ne pas se tromper et à
bien prendre la G1 v2 pas la première version de la June » — puis
d'implémenter ce qui manquait. `agoravote-g1` existait déjà (itération
4, exploratoire) et compilait depuis l'itération 6, mais n'était
câblé à aucune route API ni à aucun écran : cette itération termine le
protocole de bout en bout, backend et frontend, dans les limites
honnêtes déjà documentées par ce crate.

### Réintégration au workspace principal : nécessaire, pas cosmétique

`agoravote-g1` avait sa propre section `[workspace]` (héritage de
l'ancien environnement à toolchain figée, cf. itération 4/6). Pour que
`agoravote-api` en dépende par chemin, Cargo refuse catégoriquement
deux racines de workspace dans le même arbre de dépendances
("multiple workspace roots found in the same workspace") — pas
contournable par une astuce de configuration. Réintégration complète :
suppression de sa section `[workspace]`, ajout à `members` du
`Cargo.toml` racine, fusion de son `Cargo.lock` séparé dans celui du
workspace. Revérifié entièrement après coup : `cargo build --workspace`,
`cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, tous verts — confirme que les épinglages
de version documentés dans `crates/agoravote-g1/Cargo.toml` (posés en
itération 6) tiennent bien dans le même arbre de résolution que le
reste du projet.

### `G1Link` : second moyen de connexion, jamais un remplacement d'`Account`

Ajouté à `agoravote-core::auth` (cf. sa doc) : lie un `User` à une clé
publique Ğ1 (hex, 32 octets), jamais à une clé privée ou une phrase de
12 mots. Persisté via une nouvelle table `g1_links` (migration
`0003_g1_link.sql`), `UNIQUE` sur `public_key_hex` — même discipline
que `accounts.email` pour `Account`. `Store::insert_g1_link`/
`get_g1_link_by_public_key` suivent exactement le même patron que les
méthodes `*_account` déjà en place dans les deux backends (mémoire et
PostgreSQL), y compris la vérification manuelle d'unicité côté mémoire
pour reproduire la contrainte `UNIQUE` réelle. `StoreError::
G1PublicKeyAlreadyLinked` généralise `map_unique_violation` (jusqu'ici
câblé en dur sur `EmailAlreadyExists`) pour accepter l'erreur cible en
paramètre.

### Fraîcheur du défi sans état serveur

`Challenge::verify_freshness` (nouveau, `agoravote-g1::challenge`) lit
l'horodatage d'expiration directement dans le texte du défi reçu
(marqueur `EXP:<epoch>`, déjà embarqué par `Challenge::generate` depuis
cette itération) plutôt que d'exiger que le serveur ait mémorisé le
défi émis entre temps — cohérent avec le choix déjà fait pour les
sessions (jeton opaque, pas d'état supplémentaire à synchroniser entre
plusieurs instances de l'API). Un test dédié
(`verify_freshness_refuse_un_horodatage_falsifie_a_lavenir`) documente
explicitement l'invariant qui rend ça sûr : falsifier `EXP:` dans le
message reçu romprait la correspondance avec la signature déjà
produite sur le message d'origine — `verify_freshness` seule ne suffit
JAMAIS, elle doit toujours être appelée en plus de
`verify_signature` sur le message réellement reçu, jamais à sa place
(cf. `routes.rs::g1_verify`, qui respecte cet ordre).

### `POST /auth/g1/challenge` et `POST /auth/g1/verify`

Cf. `docs/openapi.yaml` pour le contrat complet. `g1_verify` retrouve
le `User` déjà lié à la clé publique vérifiée, ou en provisionne un
nouveau (rôle `Voter`, dans l'`organization_id` fourni par le client —
ignoré si la clé est déjà liée) lors de la toute première preuve
réussie pour cette clé, exactement comme `register`/`login` émettent
une session ensuite. Erreurs à message volontairement générique
(`unauthorized_g1`, même principe que `unauthorized_login` : ne jamais
confirmer à un attaquant qu'une clé publique donnée est déjà
enregistrée). Testé avec de VRAIES signatures sr25519 (compte de dev
Substrate standard "//Alice", jamais un vrai compte Ğ1 — même vecteur
que `signature.rs`), pas des mocks : `subxt-signer` ajouté en
dev-dependency d'`agoravote-api` pour ces 4 tests (défi frais valide,
provisionnement puis réutilisation du même utilisateur, signature
invalide → 401, défi jamais émis par le serveur → 400).

### Ce qui n'est délibérément PAS câblé : `chain-query`

`agoravote-api` ne dépend que de la feature `signature-verification`
d'`agoravote-g1`, jamais `chain-query` (vérification d'appartenance à
la toile de confiance contre un nœud Ğ1v2 réel) : cette vérification
réseau reste non testable de bout en bout dans tous les environnements
de développement utilisés jusqu'ici (accès à l'infrastructure Duniter
toujours bloqué, reconfirmé cette itération) — cf. la règle non
négociable de ce projet, jamais de code non testé présenté comme
fonctionnel. La route `/auth/g1/verify` et l'écran frontend le disent
explicitement : cette connexion prouve la possession de clé, PAS
l'appartenance à la toile de confiance — deux garanties distinctes, à
ne jamais confondre dans l'UI ou la doc.

### Écran frontend `/connexion-g1` (planche 17 de l'addendum)

Aucune intégration d'extension de portefeuille (`window.g1` ou
équivalent) : parcours manuel en deux étapes (générer un défi, coller
la clé publique et la signature obtenues du portefeuille externe), 
délibérément plus lourd qu'une future intégration native mais dont
chaque étape est réellement vérifiée côté serveur. Lié depuis
`Login.tsx`. `AuthContext` gagne `loginWithG1`, même forme que
`login`/`register`.

### Ce qui a été vérifié concrètement

- `make check` (workspace Rust complet) vert, y compris les 4 nouveaux
  tests `routes.rs` avec de vraies signatures sr25519.
- Tests d'intégration PostgreSQL réels relancés
  (`DATABASE_URL=... cargo test -p agoravote-store -- --ignored
  --test-threads=1`) : 10/10 verts, y compris les 2 tests `G1Link`
  ajoutés en itération précédente.
- `docs/openapi.yaml` mis à jour (`/auth/g1/challenge`,
  `/auth/g1/verify`, `G1ChallengeResponse`, `G1VerifyRequest`) puis
  `npm run gen:api` — `npx tsc -b` et `npm run build` verts contre le
  schéma régénéré.
- Parcours complet rejoué dans un vrai Chromium : API (mémoire) et
  frontend démarrés localement, `/connexion-g1` → génération d'un défi
  réel → signature produite hors-bande avec le compte de dev "//Alice"
  → collage clé publique/signature → vérification → redirection vers
  `/admin` avec une session valide et l'utilisateur Ğ1 auto-provisionné
  affiché ("Participant Ğ1 d43593c7…"). Captures d'écran prises à
  chaque étape.

### Ce qui reste non vérifié / non fait (honnêteté du statut)

- **`chain-query`** : cf. ci-dessus — non câblé, non testable ici.
- **Bascule par organisation** (écran "Paramètres — identité" activant/
  désactivant Ğ1 par organisation) : pas de concept backend
  d'identités optionnelles activables par organisation ; toute
  organisation accepte aujourd'hui la connexion Ğ1 si le frontend y
  mène. À revoir si le produit veut un jour cette granularité.
- **Aucune intégration d'extension de portefeuille** : cf. ci-dessus,
  parcours manuel volontairement assumé pour ce prototype.
- **`docs/SECURITY.md`** : implications de confiance envers un nœud RPC
  Ğ1v2 (pour `chain-query`, une fois câblé) documentées dans ce même
  commit — cf. ce fichier, section dédiée.

### Prochaines itérations candidates (mise à jour)

1. Confirmer les noms de stockage de `chain.rs` (`agoravote-g1`) contre
   un nœud `gdev` réel, depuis un environnement qui a accès à
   l'infrastructure Duniter — puis câbler `chain-query` à
   `/auth/g1/verify` (vérification optionnelle de toile de confiance
   par campagne).
2. `GET /campaigns` (liste) côté API, pour remplacer le palliatif de
   mémorisation locale (tableau de bord ET écran Campagnes).
3. Câbler `agoravote-stats` à une route API (statistiques descriptives
   par question) pour enrichir l'écran Analyse sans donnée fictive.
4. Permissions fines, flux d'invitation d'organisateurs, journal
   d'audit (cf. itération 3).
5. Migration `sqlx` 0.6 → 0.7/0.8 (cf. itération 6).
6. Intégration d'une extension de portefeuille Ğ1 pour éviter le
   copier-coller manuel du défi/signature.
7. Bascule d'activation de l'identité Ğ1 par organisation (écran
   "Paramètres").

## Itération 9 — correctif : ed25519, pas sr25519, pour l'identité Ğ1v2

Objectif : le porteur de projet a demandé, après l'itération 8, de
confirmer que le module d'identité Ğ1v2 ne confondait pas la Ğ1 (v1,
ancienne API BMA/HTTP) et la Ğ1v2 (Duniter v2s, Substrate). Réponse en
deux temps : l'architecture réseau ciblait bien la Ğ1v2 depuis le
départ (jamais l'ancienne API BMA) — mais des recherches web menées à
cette occasion ont révélé une erreur différente et plus grave, restée
invisible jusqu'ici : le **schéma de signature cryptographique**
utilisé (sr25519) n'est probablement pas celui des comptes/portefeuilles
Ğ1v2 réels.

### Ce qui a été découvert

`crates/agoravote-g1/src/signature.rs` (itération 6/8) vérifiait des
signatures **sr25519** via `subxt-signer`, sur la base d'une hypothèse
jamais vérifiée ("c'est le schéma par défaut de Substrate, donc de
Duniter v2"). Des recherches web (`forum.duniter.org` et
`git.duniter.org` restent bloqués par la liste blanche réseau de cet
environnement, comme pour `chain.rs` — seuls des extraits de moteur de
recherche ont pu être consultés, pas les pages elles-mêmes) ont
montré :

- Un fil de discussion officiel du forum Duniter intitulé "Use ed25519
  vs sr25519 for v2s clients", où la communauté argumente que sr25519
  (spécifique à l'écosystème Polkadot) n'apporte aucun bénéfice à
  Duniter et ferme la porte à l'interopérabilité avec les outils tiers
  qui parlent nativement ed25519.
- Une merge request du dépôt `duniter-v2s` alignant les comptes de
  développement locaux sur ed25519, "pour être cohérents avec le type
  de compte attendu".
- Un rappel que la Ğ1 historique (v1) utilisait déjà ed25519 — la
  continuité de schéma est un argument fort en faveur d'ed25519 pour
  la v2 également.

Conséquence pratique si l'erreur n'avait pas été corrigée : une vraie
signature produite par un vrai portefeuille Ğ1v2 (Ğecko, Cesium²)
aurait été **rejetée à 100 %** par `/auth/g1/verify`, alors même que
tous les tests de l'itération 8 passaient — parce qu'ils vérifiaient
des signatures sr25519 auto-produites par le même code, sans jamais
confronter l'implémentation à un vecteur externe.

### Correctif appliqué

`signature.rs` réécrit pour vérifier des signatures **ed25519** via
`ed25519-zebra` (Zcash Foundation, implémentation ed25519 indépendante
et auditée) — pas `subxt-signer`, qui ne propose d'ailleurs aucun
module `ed25519` (vérifié en lisant directement son code source dans
cet environnement : seulement `sr25519`/`ecdsa`/`eth`). Détail notable :
`ed25519-zebra` était déjà présent dans l'arbre de dépendances résolu
du workspace (tiré transitivement par `smoldot`, lui-même dépendance de
`subxt` pour `chain-query`) — l'ajouter en dépendance directe n'a donc
introduit aucune contrainte de version surprise, contrairement aux
épinglages laborieux qui avaient été nécessaires pour `subxt-signer`/
`subxt` en itération 6.

Restructuration de `Cargo.toml` : les épinglages de version hérités de
la bisection de dépendances de l'itération 6 (`zeroize`, `blake3`,
`curve25519-dalek`, `constant_time_eq`, `indexmap`, `toml_parser`,
`parity-scale-codec`, `parity-scale-codec-derive`) étaient nécessaires
à la fois pour `subxt-signer` (sr25519, désormais supprimé) et pour
`subxt` (`chain-query`) — ils sont maintenant rattachés uniquement à
`chain-query`, puisque `signature-verification` n'a plus besoin que
d'`ed25519-zebra`, une dépendance nettement plus légère. Revérifié :
`cargo build -p agoravote-g1 --features signature-verification` (léger,
rapide) ET `cargo build -p agoravote-g1 --features chain-query`
(inchangé, toujours vert) compilent tous les deux indépendamment.

Vecteurs de test : remplacés par des paires de clés ed25519 dérivées
de graines fixes et arbitraires (`ed25519-zebra` ne propose pas de
convention "compte de dev `//Alice`" comme `subxt_signer::sr25519::dev`)
— documentés comme tels dans le code, honnêtes sur le fait que ce ne
sont pas des comptes de développement Substrate officiels, juste de
vraies paires de clés EdDSA (RFC 8032) suffisantes pour prouver que
`verify_signature` vérifie une vraie signature, pas une simulation.

Propagé partout où le schéma était mentionné : `agoravote-core::auth`
(doc de `G1Link`), `agoravote-api` (dev-dependency de test, doc de
`g1_verify`, tests `routes.rs` réécrits avec de vraies signatures
ed25519), `docs/openapi.yaml` (+ régénération des types frontend),
`frontend/src/pages/G1Login.tsx`, `crates/agoravote-g1/README.md`,
`lib.rs`.

### Ce qui a été vérifié concrètement

- `cargo test -p agoravote-g1 --features signature-verification` : 13
  tests verts (dont 2 avec de vraies signatures/vérifications ed25519).
- `cargo build -p agoravote-g1 --features chain-query` : toujours vert
  après la restructuration des épinglages de `Cargo.toml`.
- `make check` (workspace complet) vert, y compris les 4 tests
  `routes.rs::g1_*` réécrits avec de vraies signatures ed25519.
- Tests d'intégration PostgreSQL réels relancés : 10/10 verts
  (inchangés par ce correctif, `G1Link` ne dépend pas du schéma de
  signature).
- `npx tsc -b`, `npx oxlint`, `npm run build` verts contre le schéma
  OpenAPI régénéré.
- Parcours complet rejoué dans un vrai Chromium avec une vraie
  signature **ed25519** cette fois (et non plus sr25519) : défi →
  signature → vérification → redirection vers `/admin` avec une
  session valide.

### Ce qui reste non vérifié (honnêteté du statut)

Cette correction s'appuie sur des recherches web (extraits de moteur
de recherche), pas sur une lecture directe de la documentation
officielle Duniter ou un test contre un vrai portefeuille Ğ1v2 (Ğecko,
Cesium²) — `forum.duniter.org` et `git.duniter.org` restent bloqués
par la liste blanche réseau de cet environnement. **À reconfirmer
avant toute mise en production**, idéalement en signant un défi réel
avec un vrai portefeuille Ğ1v2 et en vérifiant que
`POST /auth/g1/verify` l'accepte. Cf. `docs/SECURITY.md` §8.

### Prochaines itérations candidates (mise à jour)

1. **Reconfirmer le schéma ed25519 contre une source primaire**
   (documentation officielle Duniter, ou test contre un vrai
   portefeuille Ğ1v2) — priorité avant toute mise en production de ce
   module, cf. ci-dessus.
2. Confirmer les noms de stockage de `chain.rs` (`agoravote-g1`) contre
   un nœud `gdev` réel, depuis un environnement qui a accès à
   l'infrastructure Duniter — puis câbler `chain-query` à
   `/auth/g1/verify` (vérification optionnelle de toile de confiance
   par campagne).
3. `GET /campaigns` (liste) côté API, pour remplacer le palliatif de
   mémorisation locale (tableau de bord ET écran Campagnes).
4. Câbler `agoravote-stats` à une route API (statistiques descriptives
   par question) pour enrichir l'écran Analyse sans donnée fictive.
5. Permissions fines, flux d'invitation d'organisateurs, journal
   d'audit (cf. itération 3).
6. Migration `sqlx` 0.6 → 0.7/0.8 (cf. itération 6).
7. Intégration d'une extension de portefeuille Ğ1 pour éviter le
   copier-coller manuel du défi/signature.
8. Bascule d'activation de l'identité Ğ1 par organisation (écran
   "Paramètres").

## Itération 10 — correctif : `Dockerfile` ne copiait pas tous les manifestes du workspace

Objectif : le porteur de projet a lancé `./install.sh` sur sa propre
machine (ZorinOS, Docker Desktop) — la toute première fois que ce
projet était réellement construit avec Docker, cet outil n'ayant jamais
été disponible dans aucun environnement de développement utilisé
jusqu'ici (cf. CLAUDE.md). La construction a échoué :

```
error: failed to load manifest for workspace member `/build/crates/agoravote-store`
Caused by: failed to load manifest for dependency `agoravote-auth`
Caused by: failed to read `/build/crates/agoravote-auth/Cargo.toml`
Caused by: No such file or directory (os error 2)
```

### Cause

`Dockerfile` copie d'abord uniquement les manifestes (`Cargo.toml`) de
chaque crate pour profiter du cache de layers Docker (astuce standard),
avant de créer des fichiers source vides et de lancer `cargo build -p
agoravote-api` une première fois pour ne précompiler que les
dépendances tierces. Mais Cargo, pour résoudre un workspace, charge le
manifeste de **tous** les membres déclarés dans `[workspace]
members` du `Cargo.toml` racine — pas seulement ceux dont dépend le
crate ciblé par `-p`. Le `Dockerfile` ne copiait que les manifestes de
`agoravote-core`, `agoravote-voting`, `agoravote-stats`,
`agoravote-store` et `agoravote-api` : `agoravote-auth` (dépendance de
`agoravote-store`, présente depuis l'introduction de
l'authentification, cf. itération 3) et `agoravote-g1` (dépendance
directe d'`agoravote-api` depuis l'itération 8) manquaient tous les
deux — un oubli resté invisible faute d'avoir jamais pu tester une
vraie construction Docker avant aujourd'hui.

### Correctif

Ajout des `COPY crates/agoravote-auth/Cargo.toml ...` et `COPY
crates/agoravote-g1/Cargo.toml ...`, plus les répertoires/fichiers
source factices correspondants (`mkdir`/`echo ""`) et leur `touch`
dans l'étape de reconstruction finale — même patron que les crates déjà
présents.

### Ce qui a été vérifié concrètement

Le vrai `docker build` n'a pas pu être rejoué dans cet environnement :
son démon Docker, bien que techniquement démarrable ici, ne peut pas
tirer les images de base (`rust:1-slim-bookworm`, etc.) depuis Docker
Hub — bloqué par la liste blanche réseau de cet environnement (même
famille de contrainte que l'accès à l'infrastructure Duniter pour
`chain.rs`). À la place, la logique EXACTE du `Dockerfile` a été rejouée
directement avec `cargo` dans une copie isolée du dépôt (mêmes
commandes `mkdir`/`echo`/`cargo build --release -p agoravote-api`,
recompilées deux fois — une fois avec les sources factices, une fois
avec les vraies, comme le fait le `Dockerfile`) : les deux étapes
réussissent et produisent un vrai binaire `agoravote-api` exécutable.
C'est une vérification forte de la logique Cargo/workspace en jeu,
mais **pas un test du `Dockerfile` lui-même** (syntaxe Docker, layers,
étape frontend) — à confirmer par le porteur de projet en relançant
`./install.sh`.

### Prochaines itérations candidates (mise à jour)

1. Confirmer que `./install.sh` aboutit maintenant sur une machine avec
   un vrai accès Docker Hub (cf. ci-dessus, non testable depuis cet
   environnement).
2. Reste de la liste de l'itération 9 (reconfirmer le schéma ed25519,
   `chain-query`, `GET /campaigns`, etc.) — inchangée, non reprise ici.

## Itération 11 — ports hôte Docker personnalisables

Objectif : le porteur de projet a signalé un conflit de port lors de
`./install.sh` sur sa machine (ports 3000/8080 déjà utilisés par
d'autres services locaux). `docker-compose.yml` codait ces deux ports
hôte en dur, obligeant jusqu'ici à éditer directement ce fichier — la
doc existante (`docs/DOCKER.md`, "Changer le port") suggérait même
`"8080:3000"` pour l'API, qui serait entrée en collision avec le port
8080 déjà réservé au frontend dans ce même fichier (bug repéré en
corrigeant cette section).

### Correctif

`docker-compose.yml` : ports hôte remplacés par
`${AGORAVOTE_API_PORT:-3000}` et `${AGORAVOTE_FRONTEND_PORT:-8080}` —
même patron que `POSTGRES_PASSWORD` déjà présent dans ce fichier. Les
ports *conteneur* (3000 pour le binaire `agoravote-api`, 80 pour
Nginx) ne changent jamais, seuls les ports hôte (ceux qu'on ouvre dans
un navigateur) sont personnalisables. Nouveau fichier `.env.example`
(à copier en `.env`, déjà dans `.gitignore`) documentant ces deux
variables. `install.sh` source ce `.env` s'il existe (`docker compose`
le fait déjà nativement pour interpoler `docker-compose.yml`, mais un
script bash simple ne le fait pas automatiquement) pour afficher les
bonnes URLs en fin d'installation — corrige au passage un vrai manque
préexistant : le message final ne mentionnait jamais l'URL du
frontend (8080), seulement celle de l'API.

### Ce qui a été vérifié concrètement

`docker compose config` (avec puis sans `.env` présent) confirme que
les ports publiés résolvent correctement vers les valeurs
personnalisées et vers les valeurs par défaut (3000/8080) selon les
cas — vérifié dans cet environnement en démarrant `dockerd`
manuellement (le démon n'est pas actif par défaut ici, mais peut
l'être démarré ; Docker Hub, lui, reste hors d'atteinte réseau, cf.
itération 10). `bash -n install.sh` confirme la syntaxe du script
modifié.

### Prochaines itérations candidates (mise à jour)

1. Confirmer avec le porteur de projet que `./install.sh` avec un
   `.env` personnalisé (ports non conflictuels sur sa machine) aboutit
   bien jusqu'au bout.
2. Reste des listes des itérations 9 et 10, inchangées.

## Itération 12 — test de bout en bout (30 profils, 18 campagnes) et complétion des 4 types de question restants

Objectif : le porteur de projet a demandé un test de bout en bout
grandeur réelle — créer 30 comptes réels, 3 campagnes par type de
question (6 types × 3 = 18), faire répondre les autres inscrit·e·s,
afficher les résultats, capturer des écrans à chaque étape, et
vérifier la conformité au cahier des charges et aux maquettes ainsi
que la justesse mathématique de chaque résultat.

### Découverte en cours de test

En construisant un scénario couvrant les 6 `QuestionType`
(`single_choice`, `multiple_choice`, `text`, `number`, `scale`,
`ranking`), il est apparu que seuls les deux premiers étaient
réellement utilisables de bout en bout : `Vote.tsx` n'avait aucune UI
de saisie pour texte/nombre/échelle/classement (ces types étaient
créables dans le constructeur de formulaire depuis l'itération 7, mais
pas votables), et `agoravote-voting` n'a que trois méthodes
(`voting.majority`, `voting.approval`, `voting.score`) — aucune ne
convient à une question de type texte ou classement, et `voting.score`
n'était atteignable par aucun `QuestionType` réel.

Conformément à la règle du projet ("jamais de code non testé présenté
comme fonctionnel"), ceci a été signalé au porteur de projet plutôt
que contourné silencieusement. Décision prise avec lui : implémenter
complètement ces 4 types plutôt que réduire le scope du test.
Périmètre convenu, en cohérence avec le cahier des charges §5.1
("les réponses brutes doivent être distinguables des résultats
calculés") et le fait que le vote préférentiel (Condorcet/STV,
seul algorithme pertinent pour un classement) est explicitement hors
MVP (§2.2) :

- **Nombre / Échelle** : bulletin réel + statistiques descriptives
  réelles (moyenne, médiane, écart-type) via `agoravote-stats`,
  jusqu'ici une dépendance déclarée mais jamais utilisée par
  `agoravote-api`.
- **Texte / Classement** : bulletin réel, mais **aucun résultat
  calculé** — affichage des réponses brutes uniquement (pas de
  "gagnant" pour un classement sans algorithme de vote préférentiel
  implémenté ; pas de résumé statistique pertinent pour du texte
  libre).

### Ce qui a changé

Backend :
- `Ballot::for_numeric` et `Ballot::for_text` (nouveaux constructeurs,
  `crates/agoravote-core/src/ballot.rs`), en complément des
  `for_selections`/`for_scores` déjà existants.
- `CastBallotRequest` étendu (`numeric_value`, `text_value`) ;
  `cast_ballot` réécrit pour valider le bulletin **contre le type réel
  de la question** (bornes min/max, longueur max, permutation complète
  pour un classement) avant de construire le `Ballot` — corrige au
  passage un item de `docs/ROADMAP.md` ("Validation bulletin ↔ type de
  question à la soumission").
- Nouvelle route `GET /campaigns/:id/questions/:id/responses` :
  réponses brutes, distincte de `/tally` + `/results` (résultat
  calculé) — cf. cahier des charges §5.1 cité plus haut. Pour
  texte/classement, seule voie d'accès aux réponses (aucune méthode de
  vote ne s'applique) ; pour nombre/échelle, complète `/results` avec
  le détail des valeurs individuelles en plus du résumé statistique.
- Le champ `scores` de `CastBallotRequest` devient du code mort réel
  (aucun `QuestionType` ne produit de bulletin `scores` via l'API,
  `voting.score` restant un module complet mais non câblé) — marqué
  `#[allow(dead_code)]` avec commentaire, plutôt que supprimé : c'est
  une fonctionnalité MVP légitime (§8.2), déjà implémentée et testée
  dans `agoravote-voting`, simplement pas encore reliée à une route
  HTTP (lacune préexistante, hors périmètre de cette itération).
- 6 nouveaux tests d'intégration (bulletins texte/nombre/échelle/
  classement valides et invalides, route `/responses`) —
  23 tests `agoravote-api` au total, tous verts.
- `docs/openapi.yaml` mis à jour en premier (nouvelle route, DTO
  étendu, schémas `QuestionResponses`/`NumericSummary`), puis
  `cd frontend && npm run gen:api`, conformément à la règle du projet.

Frontend :
- `Vote.tsx` réécrit avec une UI de saisie par type (boutons 1..N pour
  échelle, `<input type="number">` avec bornes affichées pour nombre,
  `<textarea>` pour texte, clic-pour-classer pour classement).
- Nouveaux composants `ResponsesView`/`ResponsesPanel` (affichage des
  réponses brutes, y compris un histogramme pour l'échelle) ;
  `CampaignManage.tsx` bascule entre `TallyPanel` (choix
  unique/multiple) et `ResponsesPanel` (texte/nombre/échelle/
  classement) selon le type de question.
- `Results.tsx` réécrit pour résoudre d'abord la question (détermine
  le type, résout les libellés d'option) avant de brancher entre
  l'UI de résultat calculé existante et `ResponsesView`.
- **Bug préexistant trouvé et corrigé au passage** (pas demandé
  initialement, repéré en touchant ce code) : `Results.tsx` et
  `TallyPanel.tsx` affichaient l'identifiant brut et slugifié d'une
  option (ex. `bleu-ciel`) au lieu de son libellé humain dans les
  résultats — corrigé en résolvant via le tableau `options` de la
  question, déjà disponible dans les deux composants.

### Ce qui a été vérifié concrètement

- 30 comptes réels créés via `POST /auth/register` (pas de fixture
  DB directe), 18 campagnes (3 par type × 6 types) créées et publiées
  via l'API réelle, 522 bulletins soumis par les 29 autres inscrit·e·s
  (chaque campagne : 29 votant·e·s, un·e participant·e réservé·e au
  test navigateur ci-dessous).
- **Vérification mathématique** : un registre de vérité (`ledger`)
  tenu côté script au moment de la soumission de chaque bulletin,
  comparé bulletin par bulletin aux agrégats retournés par l'API
  (`/results` pour les 12 campagnes choix unique/multiple/nombre/
  échelle, `/responses` pour texte/classement) — correspondance exacte
  sur les 18 campagnes, y compris pour les 6 campagnes où un bulletin
  supplémentaire a ensuite été soumis en direct depuis un vrai
  navigateur (cf. ci-dessous), reconfirmé après ce vote supplémentaire.
- **Vérification visuelle** : parcours complet piloté par Chromium/
  Playwright — liste des 18 campagnes (peuplée via le palliatif
  `recentCampaigns.ts` existant, `GET /campaigns` toujours absent, cf.
  `docs/ROADMAP.md`), vote en direct sur une campagne choix unique et
  une campagne choix multiple (avant/après envoi, message de
  confirmation), dépouillement mis à jour côté organisatrice, pages de
  résultats publiques pour les 6 types de question, et les 4 nouveaux
  écrans de saisie (échelle/nombre/texte/classement) en desktop **et**
  mobile — aucune erreur console JS rencontrée sur l'ensemble du
  parcours.
- `make check` (fmt + clippy `-D warnings` + tous les tests Rust, 71
  tests au total) : vert. Tests d'intégration PostgreSQL réels
  (`cargo test -p agoravote-store -- --ignored`, base dédiée créée puis
  supprimée pour l'occasion) : 10/10 verts. `npx tsc -b`, `npx oxlint`,
  `npm run build` (frontend) : verts.

### Limites connues, non résolues par cette itération

- `voting.score` reste un module complet mais orphelin (aucune route
  ne peut produire un bulletin qui l'atteint) — cf. ci-dessus.
- `GET /campaigns` reste absent ; la liste des campagnes dépend
  toujours du palliatif `localStorage` (`recentCampaigns.ts`).
- Le classement affiche des réponses brutes (préférences individuelles
  par votant·e), pas un résultat agrégé — un algorithme de vote
  préférentiel (Condorcet/STV) reste hors MVP par choix explicite du
  cahier des charges §2.2, pas un oubli.

### Prochaines itérations candidates (mise à jour)

1. Câbler `voting.score` à un vrai chemin HTTP (nécessiterait un
   `QuestionType` de type "notation par option", absent du modèle
   actuel — décision produit avant travail technique).
2. `GET /campaigns` (déjà dans `docs/ROADMAP.md`, non repris ici).
3. Reste des listes des itérations précédentes, inchangées.

## Itération 13 — jugement majoritaire et page d'invitation (idées reprises d'un projet externe)

Objectif : le porteur de projet a signalé un projet externe
(`direct_democracy_voting`, Java/Angular, dépôt cloné localement pour
lecture du code réel — pas seulement des captures d'écran) et a
demandé d'en extraire les idées transposables, en particulier deux
rendus visuels appréciés : une page d'invitation publique avec QR
code, et un affichage de résultats "jugement majoritaire" avec des
barres empilées par mention colorée.

### Ce qui a été extrait et pourquoi

Le jugement majoritaire était déjà cité par le cahier des charges
(§6.2, §15.1 : "hors MVP mais prévu dans l'architecture") — ce n'est
donc pas une fonctionnalité hors périmètre, juste un item de la
roadmap qui restait à faire. Décision : l'implémenter comme méthode de
vote native à part entière plutôt que comme variante de `voting.score`
(déjà existant, moyenne des notes) — le jugement majoritaire calcule
une MÉDIANE, avec un départage spécifique, ce qui est un algorithme
distinct, pas un habillage du même calcul.

Point le plus intéressant du code source externe : le traitement du
vote "Ne sait pas". Une implémentation naïve qui exclurait "Ne sait
pas" du calcul de médiane ferait remonter en tête une option qui n'a
reçu QUE des "Ne sait pas" (aucune mention "opinable" ne fait
redescendre sa médiane) — un vrai biais démocratique que le projet
externe documente avoir corrigé après-coup. Reproduit ici dès le
départ : "Ne sait pas" (mention 6) est neutralisé en "Sans avis"
(mention 3) pour le calcul de la médiane, tout en restant visible et
compté dans la répartition détaillée affichée.

La page d'invitation (QR code + statistiques de participation en
direct + lien à partager) a été reprise comme nouvel écran public,
avec un compteur de bulletins BRUT (nouvelle route `GET .../ballot_count`,
distincte d'un résultat calculé — même principe de séparation que
`/responses` vs `/results`, cf. itération 12).

Idées du projet externe notées mais **non retenues cette itération**
(hors de ce qui a été demandé) : mode isoloir (vote sans influence par
les résultats en temps réel), minuteur d'expiration avec barre de
progression colorée, tri aléatoire Fisher-Yates des propositions, chat
en direct. Ajoutées à `docs/ROADMAP.md` comme candidates futures.

### Ce qui a changé

Backend :
- `QuestionType::MajorityJudgment { options }` (nouveau variant,
  `crates/agoravote-core/src/form.rs`) — chaque option reçoit une
  mention entière 1-6 par bulletin (réutilise `Ballot::scores`,
  jusqu'ici uniquement réservé pour `voting.score` sans type de
  question ne l'atteignant réellement — cf. itération 12). Contrairement
  à `voting.score`, une notation partielle À L'INTÉRIEUR D'UN bulletin
  n'a pas de sens ici : `build_ballot` exige une mention pour CHAQUE
  option de la question, ni plus ni moins.
- Nouveau module `crates/agoravote-voting/src/majority_judgment.rs`
  (`voting.majority_judgment`) : médiane des mentions neutralisées,
  départage par % de mentions à/au-dessus puis en-dessous de la
  médiane, puis par id d'option. La répartition détaillée par mention
  (comptages ET pourcentages, y compris "Ne sait pas" non neutralisé)
  est exposée via `TallyOutcome::metadata["mention_breakdown"]` — champ
  libre déjà prévu pour ce genre de détail spécifique à une méthode
  (cf. `score_scale_hint` de `voting.score`).
- Nouvelle route `GET /campaigns/:id/questions/:id/ballot_count` :
  compte brut de bulletins, sans dépouillement — ni un `ResultSet`, ni
  des réponses individuelles, juste leur nombre, pour la page
  d'invitation. Documentée dans `docs/openapi.yaml` en premier, comme
  l'exige la convention du projet.
- 10 nouveaux tests (7 dans `agoravote-voting` : classement, départage,
  neutralisation, notation partielle entre bulletins, sièges multiples,
  bulletins vides ; 3 dans `agoravote-api` : validation du bulletin,
  dépouillement + neutralisation bout en bout, `/ballot_count`) — 26
  tests `agoravote-api` et 18 tests `agoravote-voting` au total, tous
  verts.

Frontend :
- Écran de vote : une ligne par option avec les 6 boutons de mention,
  couleurs reprises À L'IDENTIQUE de la référence externe (demande
  explicite du porteur de projet) — rouge `#d32f2f` → orange `#ff9800`
  → jaune `#ffc107` → vert clair `#8bc34a` → vert foncé `#4caf50`, gris
  `#9e9e9e` pour "Ne sait pas" (`frontend/src/lib/majorityJudgmentMentions.ts`,
  nouveau fichier, même patron que `methodCopy.ts`).
- Nouveau composant `MajorityJudgmentResults.tsx` : barre horizontale
  empilée par mention, ligne de médiane (position fixe au centre, comme
  la référence — repère visuel de comparaison entre options, pas une
  position littérale du fractile), mention majoritaire et badge
  "(≥X %)", classement par médiane puis départage. Branché dans
  `Results.tsx` (page publique) et dans le nouveau
  `MajorityJudgmentPanel.tsx` (équivalent de `TallyPanel.tsx`, sans
  sélecteur de méthode puisqu'une seule s'applique ici).
- Nouvelle page publique `CampaignInvite.tsx`
  (`/campagnes/:id/questions/:id/inviter`) : QR code (nouvelle
  dépendance `qrcode`, MIT, génération côté client sans service
  externe), statistiques en direct, lien à copier — accessible depuis
  `CampaignManage.tsx`.
- `FormBuilder.tsx` : "Jugement majoritaire" ajouté à la bibliothèque
  de questions (options requises, comme choix unique/classement).

### Ce qui a été vérifié concrètement

- **50 votant·e·s réel·le·s** (comptes créés via `POST /auth/register`,
  bulletins soumis via l'API réelle) sur 3 campagnes couvrant des
  scénarios différents : répartition diverse (3 options, toutes les
  mentions représentées), neutralisation à grande échelle (une option
  ne recevant QUE des "Ne sait pas" face à une option réellement
  favorable), départage par pourcentage (deux options à médiane
  identique mais répartition différente).
- **Agent de contrôle indépendant** (demande explicite du porteur de
  projet) : une réimplémentation de l'algorithme en JavaScript, écrite
  séparément du code Rust à partir de la seule spécification (pas une
  traduction ligne à ligne), comparée bulletin par bulletin au résultat
  réellement renvoyé par l'API — correspondance exacte sur les 3
  scénarios (médiane, pourcentages, classement complet, répartition par
  mention). Complété par une revue de code indépendante (sous-agent
  dédié, sans connaissance du script de vérification ci-dessus) du
  module Rust lui-même : verdict "correct, aucun bug" sur les 5 points
  vérifiés (neutralisation, calcul de médiane, direction du
  comparateur de classement, sûreté de `total_cmp`, absence
  d'avantage résiduel pour "Ne sait pas") — avec une observation
  honnête à noter, pas un bug : la neutralisation s'applique mention
  par mention, pas option par option, donc une option avec une
  minorité d'avis réellement défavorables et une majorité de "Ne sait
  pas" peut dépasser une option notée uniformément défavorable — c'est
  la conséquence directe et voulue de la règle documentée, pas un
  défaut.
- Un 51e vote a ensuite été soumis EN DIRECT depuis un vrai navigateur
  (Chromium/Playwright) sur l'une des 3 campagnes, puis le résultat
  redépouillé et revérifié par le même agent de contrôle indépendant —
  toujours exact.
- **Non-régression complète** : le scénario de bout en bout de
  l'itération 12 (30 comptes, 18 campagnes, les 6 types de question
  préexistants) a été rejoué intégralement sur une base fraîche —
  toujours 100% correct, y compris le vote en direct navigateur sur les
  6 campagnes réservées à cet effet. Aucune régression détectée.
- Parcours visuel complet (Chromium/Playwright, desktop et mobile,
  aucune erreur console) : écran de saisie, confirmation de vote,
  dépouillement admin, résultats publics pour les 3 scénarios, page
  d'invitation avec QR code fonctionnel.
- `make check` (fmt + clippy `-D warnings` + tous les tests Rust) :
  vert. Tests d'intégration PostgreSQL réels (base dédiée) : 10/10
  verts. `npx tsc -b`, `npx oxlint`, `npm run build` (frontend) :
  verts.

### Limites connues, non résolues par cette itération

- Le classement matérialisé par le composant de résultats (React)
  re-trie côté client par `counts`/`percentages` plutôt que de recevoir
  un ordre explicite de l'API — cohérent avec le fait que `TallyOutcome`
  n'expose déjà nulle part ailleurs un "ordre total", seulement des
  `winners` (limité par `seats`) : pas une régression introduite ici,
  un choix de modèle déjà en place pour les autres méthodes.
- Les idées non retenues du projet externe (mode isoloir, minuteur
  d'expiration, tri aléatoire, chat) restent des candidates futures,
  cf. `docs/ROADMAP.md`.

### Prochaines itérations candidates (mise à jour)

1. Mode isoloir (masquer les résultats en temps réel aux votant·e·s
   tant que le scrutin n'est pas clôturé) — idée reprise du projet
   externe, non implémentée cette itération.
2. Reste des listes des itérations précédentes, inchangées.

## Itération 14 — multilinguisme de l'interface et du contenu des campagnes

Objectif : le porteur de projet a demandé de rendre l'application
capable de gérer le multilangue. Le modèle de données le permettait
déjà partiellement (§1.1 "multilinguisme natif" du cahier des charges :
`Question.prompt` et `QuestionOption.labels` sont des tables
`code langue -> texte` depuis l'origine du projet), mais deux limites
empêchaient d'en tirer parti :
1. **L'interface elle-même** (boutons, libellés, messages d'erreur —
   des centaines de chaînes) était intégralement câblée en français,
   sans mécanisme de traduction (déjà noté dans `docs/ROADMAP.md`,
   section Frontend).
2. **Le contenu des campagnes n'était en réalité PAS multilingue
   malgré le modèle** : `CreateQuestionRequest.prompt` (contrairement
   à `QuestionOption.labels`, déjà une table) était un simple `String`
   côté DTO — `create_form` le enveloppait systématiquement sous la
   seule clé "fr". Aucun organisateur ne pouvait donc réellement saisir
   un intitulé de question en plusieurs langues avant cette itération,
   même si l'option existait déjà pour les libellés d'option.

### Choix de périmètre

Deux langues au démarrage — **français** (référence, toujours requise)
et **anglais** — avec une architecture pensée pour en ajouter d'autres
sans réécriture (un fichier `translations/<code>.ts` de plus,
vérifié par le compilateur contre le dictionnaire français). Pas de
bibliothèque i18n externe : un contexte React + deux dictionnaires
statiques suffisent à l'échelle de cette application, cohérent avec le
choix déjà fait ailleurs (aucun kit UI, SVG inline pour les
graphiques — cf. `docs/DEVLOG.md`, choix répétés de dépendances
minimales).

**Persistance du choix de langue : locale au navigateur uniquement**
(`localStorage`, comme le jeton de session) — délibérément PAS
synchronisée avec `User.preferred_language` (champ qui existe déjà
dans `agoravote_core::User` depuis une itération antérieure, mais
resté jusqu'ici sans lecteur ni écrivain nulle part dans la base de
code, vérifié par recherche). Câbler cette synchronisation aurait
demandé une nouvelle route d'API (`PATCH /auth/me` ou équivalent) et
sortait du périmètre raisonnable de cette itération — noté dans
`docs/ROADMAP.md` comme prochaine étape logique.

### Ce qui a changé

Backend :
- `CreateQuestionRequest.prompt` : `String` → `HashMap<String, String>`
  (même forme que `QuestionOption.labels`, qui était déjà correcte).
  `Form::add_question` prend directement cette table plutôt que de la
  construire lui-même à partir d'une seule chaîne "fr" — c'est
  désormais à l'appelant (la route HTTP) de garantir la présence du
  français, pas à ce constructeur de portée générale de l'imposer.
- `routes.rs::create_form` valide que chaque question fournit un
  intitulé en français (`400` explicite sinon — "l'intitulé de chaque
  question doit au moins être fourni en français") : le français reste
  la langue de référence garantie, les autres sont optionnelles.
- 2 nouveaux tests d'intégration (acceptation d'un intitulé
  multilingue avec vérification de la traduction anglaise renvoyée
  telle quelle ; rejet d'un intitulé sans clé "fr") — 30 tests
  `agoravote-api` au total, tous verts. `docs/openapi.yaml` mis à jour
  en premier (schéma `CreateQuestionRequest.prompt` désormais un objet
  `additionalProperties: string`), puis régénération des types
  frontend, conformément à la convention du projet.

Frontend :
- Nouveau répertoire `src/i18n/` : `LanguageContext.tsx` (contexte
  React, fonction `t(clé, variables?)` avec interpolation simple,
  persistance `localStorage`, détection initiale : préférence déjà
  choisie -> langue du navigateur si supportée -> français par
  défaut), `translations/fr.ts` (dictionnaire de référence,
  ~180 clés) et `translations/en.ts` (même jeu de clés, vérifié par le
  compilateur via `Record<TranslationKey, string>` — une traduction
  manquante est une erreur de build, pas un texte qui reste
  silencieusement en français en production), `content.ts`
  (`resolveLocalizedText` : résout un contenu de campagne stocké selon
  la langue d'interface courante, repli langue courante -> français ->
  première traduction disponible -> chaîne vide, ne lève jamais
  d'exception), `LanguageSwitcher.tsx` (sélecteur partagé entre `Shell`
  et `AdminLayout`).
- **Toutes** les pages et tous les composants partagés ont été
  traduits (chaque chaîne française en dur remplacée par `t("clé")`) :
  écrans publics (accueil, connexion, inscription, connexion Ğ1),
  écrans admin (tableau de bord, liste de campagnes, modules, analyse,
  gestion de campagne, constructeur de formulaire), écrans de vote et
  de résultats (vote citoyen, résultats, invitation, dépouillement
  choix unique/multiple, jugement majoritaire, réponses brutes),
  composants graphiques partagés (`BarList` "— gagnant"), libellés des
  méthodes de vote natives (`methodCopy.ts`, devenu une fonction
  `getMethodCopy(id, t)` plutôt qu'un dictionnaire figé) et des
  mentions du jugement majoritaire (`majorityJudgmentMentions.ts`,
  couleurs restées fixes — indépendantes de la langue — mais libellés
  désormais résolus via `t()`).
- **Contenu des campagnes** : `Vote.tsx`, `Results.tsx`,
  `TallyPanel.tsx`, `MajorityJudgmentPanel.tsx`, `ResponsesPanel.tsx`,
  `ResponsesView.tsx`, `CampaignInvite.tsx` et `Analysis.tsx`
  résolvent désormais `prompt`/`labels` via `resolveLocalizedText`
  plutôt que de lire `.fr` en dur — un intitulé de question ou un
  libellé d'option s'affiche dans la langue d'interface courante si une
  traduction existe, sinon retombe proprement sur le français.
- `FormBuilder.tsx` : un second champ optionnel "Traduction anglaise"
  à côté de chaque intitulé de question et chaque libellé d'option
  (le français reste seul obligatoire) — `localizedRecord()` construit
  la table `{fr, en?}` envoyée à l'API, n'incluant "en" que si
  l'organisateur l'a réellement renseigné (un champ anglais vide ne
  produit pas une traduction vide qui casserait le repli).

### Ce qui a été vérifié concrètement

- Parcours complet piloté par Chromium/Playwright : bascule
  français/anglais sur l'accueil, persistance du choix après
  rechargement de page (`localStorage`), inscription en anglais,
  création d'une campagne bilingue via l'API réelle (intitulé de
  question ET libellés d'option traduits), vote et page de résultats
  publique affichant le contenu de campagne en anglais (pas seulement
  le chrome de l'interface — vérifié explicitement : le `<h1>` de
  l'écran de vote affiche bien "What is your favorite color?", pas
  l'intitulé français), retour au français avec repli correct, tableau
  de bord admin bilingue avec sélecteur dans la nav latérale, tiroir de
  navigation mobile avec sélecteur inclus. Aucune erreur console sur
  l'ensemble du parcours.
- Détail notable : Chromium headless détecte `navigator.language =
  "en-US"` dans cet environnement — l'application choisit donc
  correctement l'anglais par défaut au premier chargement (repli
  navigateur fonctionnel, pas juste un repli français câblé en dur).
- `make check` (fmt + clippy `-D warnings` + tous les tests Rust) :
  vert. `npx tsc -b`, `npx oxlint`, `npm run build` (frontend) : verts
  — en particulier, `tsc` confirme que `en.ts` fournit bien une
  traduction pour chacune des clés définies dans `fr.ts` (le contrat de
  type `Record<TranslationKey, string>` aurait fait échouer la
  compilation sinon).

### Limites connues, non résolues par cette itération

- `User.preferred_language` reste un champ non lu/non écrit : le choix
  de langue ne se synchronise pas entre appareils (cf. "Choix de
  périmètre" ci-dessus).
- Seules deux langues sont fournies (français, anglais) — l'ajout d'une
  troisième langue est prévu par l'architecture mais pas encore fait.
- Le contenu déjà existant en base (campagnes créées avant cette
  itération) n'a par construction qu'une clé "fr" dans `prompt`/
  `labels` : rien à migrer (le repli vers le français gère nativement
  ce cas), mais ces anciennes campagnes resteront françaises tant que
  personne n'y ajoute de traduction — comportement attendu, pas un bug.

### Prochaines itérations candidates (mise à jour)

1. Synchroniser `User.preferred_language` (nouvelle route de mise à
   jour du profil) pour que le choix de langue suive l'utilisateur
   entre appareils, pas seulement ce navigateur.
2. Ajouter une troisième langue pour valider que l'architecture tient
   au-delà de deux (aucun changement de code attendu ailleurs qu'un
   nouveau fichier `translations/<code>.ts`).
3. Reste des listes des itérations précédentes, inchangées.

## Itération 15 — nouveaux ports hôte Docker par défaut (3000/8080 → 4000/4080)

Objectif : le porteur de projet a signalé un nouvel échec de
`./install.sh` sur sa machine (Mac Mini, Docker Desktop), cette fois à
la toute dernière étape (`docker compose up`) : `Bind for
0.0.0.0:8080 failed: port is already allocated`. Contrairement au
conflit de l'itération 11 (un autre service local occupait
3000/8080), la sortie de `sudo ss -tulnp` fournie montre que le
coupable est ici **Docker Desktop lui-même** — son processus
`com.docker.back` écoute déjà sur 3000 ET 8080 (ainsi que 8090, 8091,
5002, 5678, 5433, 34289) pour son propre usage interne. Le mécanisme
de personnalisation posé en itération 11 (`AGORAVOTE_API_PORT`/
`AGORAVOTE_FRONTEND_PORT` via `.env`) fonctionnait déjà et n'a pas
été touché — mais des valeurs par défaut qui collisionnent avec
Docker Desktop lui-même, sur une plateforme (macOS/Linux avec Docker
Desktop) probablement fréquente parmi les futurs utilisateurs de ce
projet, sont un mauvais choix de défaut, pas seulement "de malchance"
pour un utilisateur en particulier.

### Correctif

Valeurs par défaut changées, partout où `docker-compose.yml` et
`install.sh` les codaient : `AGORAVOTE_API_PORT` 3000 → 4000,
`AGORAVOTE_FRONTEND_PORT` 8080 → 4080. Choisies en vérifiant qu'aucune
des deux ne figure dans la liste complète des ports fournie par le
porteur de projet (`ss -tulnp` : 3000, 8080, 8090, 8091, 5002, 5678,
5433, 34289, 6341, 32700, 3306, 631, 33060, 139, 25, 445, 80, 1716,
35587, et les entrées UDP/IPv6) — et déjà utilisées comme exemple dans
`docs/DOCKER.md` avant cette itération, donc déjà un choix
implicitement vérifié pour ne pas entrer en collision avec les ports
*conteneur* du projet lui-même (3000/80, qui eux ne changent jamais).

Seuls les ports **hôte Docker** ont changé — strictement ce que le
porteur de projet a demandé ("modifier le docker de sorte qu'il en
utilise d'autres") :
- `docker-compose.yml` (défauts `${AGORAVOTE_API_PORT:-4000}`/
  `${AGORAVOTE_FRONTEND_PORT:-4080}`, commentaires mis à jour).
- `.env.example` (mêmes nouvelles valeurs, commentaire expliquant la
  raison : collision fréquente avec Docker Desktop).
- `install.sh` (mêmes valeurs de repli pour l'affichage final des
  URLs).
- `docs/DOCKER.md` : tous les exemples `curl`/navigateur mis à jour ;
  la section "Changer le port" reformulée puisque 4000/4080
  deviennent le défaut (l'exemple de personnalisation utilise
  désormais 5000/5080, pour ne pas laisser un exemple identique au
  défaut) ; ligne du tableau de dépannage mise à jour.
- `CLAUDE.md` (commande `docker compose up -d` de la section
  "Commandes essentielles") et `README.md` (section démarrage rapide
  Docker uniquement).

Explicitement **non touché**, car hors de la portée de la demande
(ports *conteneur*, pas ports *hôte*, ou workflow local sans Docker) :
`Dockerfile` (`EXPOSE 3000`), `frontend/nginx.conf` (`listen 80`),
`crates/agoravote-api/src/main.rs` (adresse d'écoute réelle du
binaire, partagée par le mode Docker ET le mode `cargo run` local —
c'est précisément parce que ce port ne bouge jamais que le mécanisme
de port hôte personnalisable fonctionne), les sections `README.md`
décrivant `cargo run -p agoravote-api` en local (toujours port 3000,
sans rapport avec Docker), `docs/SECURITY.md` (port 3000 mentionné
génériquement pour le conseil de reverse proxy TLS), et l'URL
d'exemple `http://localhost:3000` dans `docs/openapi.yaml` (référence
illustrative du contrat d'API, pas une adresse que quiconque ouvre
réellement pour ce déploiement).

### Ce qui a été vérifié concrètement — et ce qui ne l'a pas été

Vérification faite : relecture croisée de tous les fichiers changés
pour confirmer la cohérence des nouvelles valeurs entre eux (mêmes
deux nombres partout où un défaut hôte apparaît), et recherche
exhaustive (`grep -rn "3000\|8080"`) dans tout le dépôt pour s'assurer
qu'aucune occurrence pertinente au port hôte Docker n'a été oubliée.

**Non vérifié : un `docker compose up` réel avec ces nouveaux
défauts.** Comme lors des itérations précédentes touchant à Docker
dans cet environnement (cf. itération 6, itération 10), l'accès
réseau aux couches d'images Docker Hub reste bloqué par la politique
réseau de ce bac à sable — `docker compose build`/`up` n'a pas pu être
rejoué ici. C'est un changement de configuration pur (aucune ligne de
code Rust ni TypeScript modifiée), donc `make check` et la chaîne de
vérification frontend (`tsc`/`oxlint`/`build`) ne sont pas affectés
par construction et n'ont rien à révéler sur ce point précis — mais la
seule vérification qui compte réellement ici (que `docker compose up`
réussisse enfin sur la machine du porteur de projet) reste à faire par
lui.

### Prochaines itérations candidates (mise à jour)

1. Confirmer avec le porteur de projet que `./install.sh`/
   `docker compose up -d` aboutit désormais jusqu'au bout sur sa
   machine avec les nouveaux défauts 4000/4080.
2. Reste des listes des itérations précédentes, inchangées.
