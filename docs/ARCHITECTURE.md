# Architecture — correspondance code ↔ cahier des charges

Ce document fait le lien entre chaque fichier du code et la section du
cahier des charges (`Cahier_des_charges_AgoraVote_v0_2.docx`) qui le
justifie. Objectif : qu'un contributeur qui lit une décision de code
étrange puisse remonter à l'exigence qui l'explique, et inversement.

## Vue d'ensemble

```
                     ┌────────────────────────┐
                     │      agoravote-api      │   §12 — API HTTP
                     │  (Axum, async)           │
                     └───────────┬─────────────┘
                                 │ assemble
       ┌──────────────────┬──────┼──────┬──────────────────┐
       │                  │      │      │                  │
┌──────▼─────────┐ ┌──────▼───────┐ ┌───▼──────────┐ ┌─────▼───────────┐
│ agoravote-voting │ │agoravote-core │ │agoravote-stats│ │ agoravote-store  │
│ §6.2, §7, §8.2    │ │  §5, §6        │ │ §8.3, §8.4     │ │  §12.2            │
│ méthodes de vote  │ │  modèle +      │ │ statistiques   │ │  persistance       │
│ natives           │ │  contrats      │ │ descriptives   │ │  (mémoire/Postgres)│
└──────┬────────────┘ └───────────────┘ └───────────────┘ └─────────┬────────┘
       │ dépend de                                                  │ dépend de
       └──────────────────────────► agoravote-core ◄────────────────┘
```

`agoravote-stats` ne dépend d'aucun autre crate du projet : c'est
volontaire (cf. doc de `agoravote-stats/src/lib.rs`) — le moteur
statistique ne doit connaître ni les bulletins, ni les campagnes,
uniquement des vecteurs de nombres ou de catégories.
`agoravote-store` ne dépend que de `agoravote-core` (les entités
sérialisables), pas de `agoravote-voting`/`agoravote-stats` : la
persistance ne doit rien savoir des méthodes de calcul.

## `agoravote-core` — §5 (modèle) et §6 (contrats)

| Fichier | Entité / contrat | Section du cahier des charges |
|---|---|---|
| `organization.rs` | `Organization` | §5, tableau "Modèle de données conceptuel" |
| `user.rs` | `User`, `Role` | §5, §13 ("RBAC / permissions fines" — ébauche) |
| `campaign.rs` | `Campaign`, `CampaignStatus` | §5, §5.1 (immutabilité après clôture), §18 |
| `form.rs` | `Form`, `Question`, `QuestionType`, `QuestionOption` | §5, §6.2 (types de question), §7 |
| `ballot.rs` | `Ballot` | §5, §5.1, §13 (séparation identité/bulletin) |
| `module.rs` | `Module`, `ModuleManifest`, `ModuleKind` | §6 (manifeste YAML), §6.2 |
| `voting_method.rs` | `VotingMethod`, `VotingParams`, `TallyOutcome`, `VotingError` | §6, §7, §8.1 |
| `result.rs` | `ResultSet` | §5, §5.1, §9 |
| `audit.rs` | `AuditEvent` | §5, §13 |

**Règle de conception centrale** (voir `lib.rs`) : `agoravote-core` ne
dépend d'aucune méthode de vote, moteur statistique ou visualisation
concrète. Il ne définit que des traits. C'est la traduction littérale
de l'exigence du §4 :

> « un changement de visualisation ne doit jamais modifier le résultat
> électoral ; un changement de méthode de vote doit produire un
> nouveau résultat calculé à partir des mêmes données »

## `agoravote-voting` — §6.2, §7, §8.2

| Fichier | Module | Entrée attendue | Notes d'implémentation |
|---|---|---|---|
| `majority.rs` | `voting.majority` | `single_choice` | Quorum calculé sur `eligible_voters`, gère les égalités (plusieurs gagnants) |
| `approval.rs` | `voting.approval` | `multiple_choice` | `seats` détermine combien d'options gagnent |
| `score.rs` | `voting.score` | `score` | Moyenne par option, gère les notations partielles |
| `registry.rs` | `VotingMethodRegistry` | — | Table `id → Arc<dyn VotingMethod>`, préfigure le registre mixte natif/WASM du §6.1 |

Chaque méthode est un type unitaire (`struct Foo;`) sans état : voir
la note dans `majority.rs` sur la pureté exigée de
`VotingMethod::tally` (mêmes bulletins + mêmes paramètres ⇒ même
résultat, à chaque appel — condition nécessaire pour qu'un
dépouillement soit auditable, §13).

**Comment ajouter une méthode de vote** :
1. Créer `crates/agoravote-voting/src/ma_methode.rs`.
2. Implémenter `Module` (le manifeste) et `VotingMethod` (le calcul).
3. L'enregistrer dans `VotingMethodRegistry::with_builtin_methods()`
   (`registry.rs`) — c'est le seul endroit à modifier pour la
   "brancher" sur le reste du système.
4. Ajouter des tests contre des bulletins connus, si possible en
   confrontant le résultat à une référence externe (Condorcet PHP,
   OpenSTV — cf. cahier des charges §2.2) pour les méthodes complexes.

## `agoravote-stats` — §8.3, §8.4

| Fichier | Fonctions | Section |
|---|---|---|
| `descriptive.rs` | `mean`, `median`, `mode`, `variance`, `std_dev`, `quartiles`, `frequency_table`, `proportions`, `DescriptiveSummary` | §8.3 |
| `crosstab.rs` | `cross_tabulation` | §8.4 |

Choix méthodologiques documentés directement dans le code (et donc
dans `cargo doc`) car ils affectent l'audit d'un résultat :
- **Variance d'échantillon** (division par n-1), pas variance de
  population — les répondants sont traités comme un échantillon.
- **Quartiles par "exclusive median method"** — un choix parmi
  plusieurs conventions existantes (cf. commentaire dans
  `descriptive.rs`), documenté pour éviter toute ambiguïté en cas de
  contestation d'un résultat.

## `agoravote-auth` — §13

| Fichier | Rôle |
|---|---|
| `password.rs` | `hash_password`/`verify_password` (Argon2id, format PHC) |
| `session.rs` | `issue_session` (jeton 256 bits + empreinte SHA-256), `hash_token`, `SESSION_DURATION` |

Ne fait aucune I/O (pas de dépendance à `agoravote-store`) : ce crate
transforme un mot de passe en empreinte et produit/vérifie des jetons,
il ne sait ni lire ni écrire un compte — même séparation que
`agoravote-voting` (calcul pur) vis-à-vis de la persistance. Les types
`Account`/`Session` eux-mêmes vivent dans `agoravote-core` (cf.
`auth.rs` de ce crate), pour que `agoravote-store` puisse les
persister sans dépendre de la logique de hachage.

**Jeton de session jamais stocké en clair** : `issue_session` renvoie
le jeton en clair (à donner au client une fois) et une `Session` qui
ne contient que son empreinte SHA-256 — cf. le commentaire détaillé de
`agoravote_core::auth::Session` pour la justification complète
(protection en cas de fuite de la base, jeton opaque plutôt que JWT
pour permettre une révocation immédiate).

## `agoravote-store` — §12.2

| Fichier | Rôle |
|---|---|
| `lib.rs` | Type `Store` (enum `Memory`/`Postgres`), `StoreError` |
| `memory.rs` | `MemoryStore` — backend en RAM, utilisé par défaut et par tous les tests unitaires du reste du workspace |
| `postgres.rs` | `PgStore` — backend PostgreSQL réel (`sqlx`), transactions avec verrou de ligne pour les mises à jour |
| `migrations/0001_init.sql` | Schéma SQL (JSONB) : campagnes, formulaires, bulletins, résultats |
| `migrations/0002_auth.sql` | Schéma SQL (§13) : utilisateurs, comptes (email unique), sessions (clé = empreinte du jeton) |
| `migrations/0003_g1_link.sql` | Schéma SQL (addendum v0.3) : liens `User` ↔ clé publique Ğ1 (`public_key_hex` unique) — cf. section `agoravote-g1` ci-dessous |
| `tests/postgres_integration.rs` | 10 tests `#[ignore]` exécutés contre un vrai PostgreSQL (voir l'en-tête du fichier pour la commande) |

**Pourquoi un `enum` plutôt qu'un trait `dyn Store` ?** Rust 1.75 (la
toolchain utilisée pour développer ce crate, cf. `Cargo.toml` racine)
ne permet pas nativement les fonctions `async` dans un trait objet
sans dépendance supplémentaire (`async-trait`). Avec deux backends
seulement, un `enum` avec un `match` par méthode est plus simple et
tout aussi extensible.

**Comment `agoravote-api` choisit son backend** : `main.rs` regarde la
variable d'environnement `DATABASE_URL`. Présente → PostgreSQL (avec
migration automatique) ; absente → mémoire, avec un avertissement
explicite dans les logs (« TOUTES LES DONNÉES SERONT PERDUES »)
plutôt qu'un repli silencieux.

## `agoravote-api` — §12

| Fichier | Rôle |
|---|---|
| `main.rs` | Démarrage du serveur Axum, choix du backend de persistance |
| `state.rs` | `AppState` — assemble `agoravote_store::Store` et le registre des méthodes de vote |
| `auth.rs` | Extracteurs axum `AuthUser`/`OptionalAuthUser` (§13), vérification de rôle |
| `dto.rs` | Structures JSON de requête/réponse, séparées du modèle interne |
| `routes.rs` | Handlers HTTP (tous asynchrones vis-à-vis du store) + table de correspondance routes ↔ écrans (§10.1) en tête de fichier |

## `agoravote-g1` — addendum v0.3 (§ Identité décentralisée)

**Membre du workspace principal depuis l'itération 8** (cf.
`docs/DEVLOG.md`) — sa toolchain compile désormais dans le même
`cargo build --workspace` que le reste du projet, ce qui a permis à
`agoravote-api` d'en dépendre par chemin (Cargo refuse qu'un membre de
workspace dépende d'un crate qui déclare son propre workspace séparé).
Module optionnel de preuve de possession de compte Ğ1v2 (signature
ed25519) et de vérification d'adhésion à la toile de confiance (requête
à la chaîne Duniter v2) — cf. `crates/agoravote-g1/README.md` et
`docs/G1_INTEGRATION.md` pour le détail complet. **Corrigé sr25519 ->
ed25519 en itération 9** (cf. `docs/DEVLOG.md`, `docs/SECURITY.md`
§8) : le schéma d'origine (itération 8) était une hypothèse jamais
vérifiée, probablement fausse pour les comptes Ğ1v2 réels.

| Fichier | Statut |
|---|---|
| `challenge.rs` | Compilé et testé (7 tests verts) — génération de défi et `verify_freshness` (fraîcheur vérifiable sans état serveur, horodatage embarqué dans le texte signé), pur, sans dépendance réseau |
| `signature.rs` | Compilé et testé (6 tests verts, dont un vecteur ed25519 connu) — feature `signature-verification`, activée par `agoravote-api` (cf. son `Cargo.toml`), via `ed25519-zebra` (pas `subxt-signer`, qui ne propose pas de module ed25519) |
| `chain.rs` | Compilé (feature `chain-query`) mais non vérifié à l'exécution — réseau Duniter/Ğ1 toujours bloqué dans cet environnement, noms de stockage non confirmés. **Non activée par `agoravote-api`** : `/auth/g1/verify` ne prouve donc que la possession de clé, jamais l'appartenance à la toile de confiance — cf. `docs/SECURITY.md` §8. |

**Câblage HTTP** (itération 8) : `POST /auth/g1/challenge` et
`POST /auth/g1/verify` dans `agoravote-api/src/routes.rs` (cf. table de
la section `agoravote-api` ci-dessus et `docs/openapi.yaml` pour le
contrat complet), persistance via `G1Link` (`agoravote-core::auth`) et
`Store::insert_g1_link`/`get_g1_link_by_public_key` (`agoravote-store`,
même patron que `Account`/`get_account_by_email`).

## Frontend — `frontend/`

Interface React (Vite, TypeScript, Tailwind v4), hors du workspace
Cargo (projet Node séparé). Consomme l'API via `src/api/client.ts`,
dont les types (`src/api/schema.ts`) sont **générés automatiquement**
depuis `docs/openapi.yaml` (`npm run gen:api`) — jamais dupliqués à la
main, pour qu'un changement d'API côté Rust se traduise en erreur de
compilation TypeScript plutôt qu'en bug silencieux à l'exécution.

| Fichier/dossier | Rôle |
|---|---|
| `src/api/` | Client HTTP typé + schéma généré |
| `src/auth/AuthContext.tsx` | Session React (login/register/logout, jeton en `localStorage`, confirmation du jeton via `GET /auth/me` au chargement) |
| `src/components/AdminLayout.tsx` | Disposition avec nav latérale pour les écrans admin (planches 1-3/5-6 du §10) — tiroir superposé sous 1024px, cf. itération 7 |
| `src/components/Shell.tsx` | Disposition nav du haut pour les écrans publics/citoyens (accueil, connexion, vote, résultats) |
| `src/components/charts.tsx` | Jauge de participation, liste à barres, donut — SVG inline, palette de la skill dataviz (cf. `index.css`) |
| `src/components/icons.tsx` | Icônes trait dessinées à la main (pas de dépendance externe) |
| `src/components/FormBuilder.tsx` | Bibliothèque de questions (six types déjà acceptés par l'API) + canevas |
| `src/components/` (autres) | UI de base (`ui.tsx`), panneau de dépouillement — choix unique/multiple (`TallyPanel.tsx`) —, panneau/vue de réponses brutes — texte/nombre/échelle/classement (`ResponsesPanel.tsx`/`ResponsesView.tsx`, itération 12) |
| `src/pages/` | Un fichier par écran (cf. `frontend/README.md` pour la table complète) — inclut depuis l'itération 7 `CampaignsList.tsx`, `ModulesPage.tsx` et `Analysis.tsx`, et depuis l'itération 8 `G1Login.tsx` (planche 17 de l'addendum, `/connexion-g1`) |
| `src/lib/methodCopy.ts` | Libellés/descriptions éditoriales des méthodes de vote natives, partagés entre `ModulesPage` et `TallyPanel` |
| `DESIGN.md` | Direction visuelle et sa justification |

Compilation TypeScript, lint et build de production vérifiés verts, et
**testé dans un vrai navigateur** (Chromium) à deux reprises : itération
6 (première vérification, a révélé le bug `/auth/me`) et itération 7
(refonte sur les planches UX fournies par le porteur de projet, a
révélé et corrigé deux bugs de responsive mobile — cf. `docs/DEVLOG.md`
pour le détail des deux itérations).

## Ce que ce prototype ne fait pas encore

Voir la section correspondante du `README.md` à la racine — elle
reprend le §21 du cahier des charges ("Travaux restant à mener") et
indique lesquels de ces travaux sont déjà entamés par ce dépôt.
