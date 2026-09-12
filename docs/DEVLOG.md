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
