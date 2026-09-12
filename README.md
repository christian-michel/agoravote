# AgoraVote — cœur applicatif (prototype)

> Implémentation de démarrage du cahier des charges v0.2 (« Plateforme
> libre de sondage, consultation, vote et analyse »). Ce dépôt contient
> le **noyau Rust** : modèle de données, moteur de vote modulaire,
> moteur statistique, une **authentification** (comptes, sessions,
> rôles), une **persistance PostgreSQL réelle** (avec repli en mémoire
> pour le mode local), une API HTTP documentée par contrat OpenAPI
> (`docs/openapi.yaml`), et un **frontend React** (`frontend/`) qui la
> consomme. Voir [« Ce qui n'est pas fait »](#ce-qui-nest-pas-encore-fait)
> pour les limites connues de chaque partie.
> Une identité décentralisée optionnelle (Ğ1v2, addendum v0.3) est
> câblée de bout en bout (`POST /auth/g1/challenge`/`verify`, écran
> `/connexion-g1`) : elle ne prouve que la possession de clé, pas
> l'appartenance à la toile de confiance — voir
> [`docs/G1_INTEGRATION.md`](docs/G1_INTEGRATION.md) et
> [`docs/SECURITY.md`](docs/SECURITY.md) §8.

**Vous reprenez ce projet ?** Lisez [`CLAUDE.md`](CLAUDE.md) (contexte
et conventions, pensé pour un agent de code mais utile à toute
personne qui reprend le projet) et [`docs/ROADMAP.md`](docs/ROADMAP.md)
(priorités actuelles).

## Frontend

Une interface React consomme l'API (`frontend/`, cf. son
[`README.md`](frontend/README.md) et [`DESIGN.md`](frontend/DESIGN.md)
pour la direction visuelle). Compilée, testée (TypeScript, lint, build
de production) et désormais **vérifiée dans un vrai navigateur**
(cf. [`docs/DEVLOG.md`](docs/DEVLOG.md), itération 6 — parcours complet,
desktop et mobile, zéro erreur console) :

```bash
docker compose up -d
# puis ouvrir http://localhost:8080
```

## Pourquoi ce découpage en crates ?

Le cahier des charges pose une exigence d'architecture (§4) : un
changement de méthode de vote ou de visualisation ne doit jamais
modifier silencieusement un autre composant. On traduit cette règle en
**frontière de compilation**, pas seulement en convention :

```
agoravote-core     → modèle de données + contrats (traits), aucune dépendance métier
agoravote-voting    → méthodes de vote natives, ne dépend QUE de core
agoravote-stats     → statistiques descriptives, ne dépend de rien (agnostique du domaine)
agoravote-store     → persistance (mémoire ou PostgreSQL), ne dépend QUE de core
agoravote-api       → API HTTP, assemble core + voting + stats + store
```

`agoravote-voting` ne peut pas dépendre d'un détail interne d'une
autre méthode de vote : le seul point de contact entre modules est le
trait `VotingMethod` défini dans `agoravote-core`. C'est la même
logique qui, demain, permettra de charger des modules communautaires
en WASM (cahier des charges §6.1) sans toucher au noyau.

Voir [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) pour le détail
fichier par fichier et la correspondance avec chaque section du cahier
des charges, [`docs/DEVLOG.md`](docs/DEVLOG.md) pour le journal des
décisions prises pendant cette première itération,
[`docs/LOGGING.md`](docs/LOGGING.md) pour la stratégie de logs et la
détection des erreurs silencieuses, [`docs/SECURITY.md`](docs/SECURITY.md)
pour la revue de sécurité, [`docs/DEVELOPMENT_WORKFLOW.md`](docs/DEVELOPMENT_WORKFLOW.md)
pour le process de travail au quotidien, et [`docs/DOCKER.md`](docs/DOCKER.md)
pour le déploiement via Docker.

## Démarrer — avec Docker (recommandé)

```bash
./install.sh
# puis, une fois terminé :
curl http://localhost:3000/health
```

Voir [`docs/DOCKER.md`](docs/DOCKER.md) pour le détail (Zorin OS 18 et
toute distribution Debian/Ubuntu), et `./uninstall.sh` pour tout
retirer proprement.

## Démarrer — sans Docker (Rust en local)

Prérequis : Rust (édition 2021 minimum ; testé avec rustc 1.75 via le
paquet `rustc`/`cargo` d'Ubuntu — les dépendances sont épinglées en
conséquence, voir la note dans `Cargo.toml`).

```bash
# Compiler tout le workspace
cargo build --workspace

# Lancer toute la suite de tests (44 tests actuellement)
cargo test --workspace

# Ou, en un coup : formatage + lints stricts + tests (ce que la CI vérifie)
make check

# Lancer le serveur de démonstration (http://localhost:3000)
# — sans DATABASE_URL : backend en mémoire (données perdues à l'arrêt)
cargo run -p agoravote-api

# — avec DATABASE_URL : backend PostgreSQL persistant (nécessite un
#   PostgreSQL accessible ; le plus simple reste `./install.sh`, qui
#   en fournit un via Docker automatiquement)
DATABASE_URL=postgres://user:pass@localhost/agoravote cargo run -p agoravote-api
```

## Parcours de démonstration (bout en bout)

Le scénario ci-dessous correspond exactement à la question illustrée
sur la planche "4. Vote citoyen" du cadrage UX : une question à choix
unique sur les priorités budgétaires d'une commune.

```bash
BASE=http://localhost:3000

# 1. Découvrir les méthodes de vote installées (écran "16. Modules")
curl -s $BASE/modules | python3 -m json.tool

# 2. S'inscrire (§13) — crée un compte organisateur et une session
REGISTER=$(curl -s -X POST $BASE/auth/register -H 'Content-Type: application/json' -d '{
  "organization_id": "11111111-1111-1111-1111-111111111111",
  "email": "organisatrice@exemple.fr",
  "password": "un-mot-de-passe-solide",
  "display_name": "Organisatrice"
}')
TOKEN=$(echo "$REGISTER" | python3 -c "import sys,json;print(json.load(sys.stdin)['token'])")
# (ou, si le compte existe déjà : POST $BASE/auth/login avec email+password)

# 3. Créer une campagne (écrans "01/02") — nécessite le jeton
CAMPAIGN=$(curl -s -X POST $BASE/campaigns -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{
  "organization_id": "11111111-1111-1111-1111-111111111111",
  "title": "Budget participatif 2026"
}')
CAMPAIGN_ID=$(echo "$CAMPAIGN" | python3 -c "import sys,json;print(json.load(sys.stdin)['id'])")

# 4. Créer le formulaire (écran "03. Éditeur de formulaire") — nécessite le jeton
FORM=$(curl -s -X POST $BASE/campaigns/$CAMPAIGN_ID/form -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{
  "questions": [{
    "prompt": "Quelle est votre priorité pour cette année ?",
    "type": "single_choice",
    "required": true,
    "options": [
      {"id": "transport", "labels": {"fr": "Améliorer les transports publics"}},
      {"id": "espaces_verts", "labels": {"fr": "Développer les espaces verts"}},
      {"id": "associatif", "labels": {"fr": "Soutenir le tissu associatif"}}
    ]
  }]
}')
QUESTION_ID=$(echo "$FORM" | python3 -c "import sys,json;print(json.load(sys.stdin)['questions'][0]['id'])")

# 5. Publier (écran "02. Campagnes") — nécessite le jeton
curl -s -X POST $BASE/campaigns/$CAMPAIGN_ID/publish -H "Authorization: Bearer $TOKEN"

# 6. Voter (écran "09. Vote citoyen") — SANS jeton : vote anonyme, comme
#    n'importe quel citoyen non connecté (§3.2, un sondage public n'exige
#    pas toujours une identité). Avec un jeton, le vote serait nominatif
#    et un second vote sur la même question serait refusé (anti double-vote).
curl -s -X POST $BASE/campaigns/$CAMPAIGN_ID/questions/$QUESTION_ID/ballots \
  -H 'Content-Type: application/json' -d '{"selections":["transport"]}'

# 7. Dépouiller avec la méthode "majorité simple" (écran "06. Configuration du scrutin") — nécessite le jeton
curl -s -X POST $BASE/campaigns/$CAMPAIGN_ID/questions/$QUESTION_ID/tally \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"voting_method_id":"voting.majority","eligible_voters":10,"quorum":0.5,"seats":1}' \
  | python3 -m json.tool

# 8. Consulter le résultat stocké (écran "11. Résultats en direct") — public
curl -s $BASE/campaigns/$CAMPAIGN_ID/questions/$QUESTION_ID/results | python3 -m json.tool
```

Ce parcours a été exécuté et vérifié pendant le développement contre
le serveur réel avec PostgreSQL, y compris les cas d'erreur :
création de campagne sans jeton (`401`), connexion avec mauvais mot de
passe (`401`), double inscription avec le même email (`400`), second
vote d'un participant identifié sur la même question (`400`), vote
refusé après clôture, publication refusée sans formulaire, méthode de
vote inconnue refusée — voir `docs/DEVLOG.md` pour le détail complet.

## Ce qui est fait

- **Modèle de données** (§5) : `Organization`, `User`/`Role`,
  `Campaign` (avec cycle de vie), `Form`/`Question`/`QuestionType`,
  `Ballot`, `ResultSet`, `AuditEvent` (défini mais pas encore branché
  sur l'API).
- **Contrat de module et méthodes de vote** (§6, §7, §8.2) :
  `voting.majority`, `voting.approval`, `voting.score`, chacune testée
  unitairement contre des jeux de bulletins connus.
- **Moteur statistique** (§8.3, §8.4) : moyenne, médiane, mode,
  variance/écart-type, quartiles, table de fréquence, tableau croisé.
- **Authentification** (§13) : inscription/connexion avec mots de
  passe hachés Argon2id, sessions par jeton (stocké haché SHA-256,
  jamais en clair), routes d'administration protégées par rôle
  `Organizer`/`Admin`, anti double-vote pour tout participant
  identifié. Vérifié par 9 tests d'intégration + un parcours complet
  contre le serveur réel (cf. `docs/DEVLOG.md`).
- **Persistance** (§12.2) : backend PostgreSQL réel
  (`agoravote-store`), avec repli en mémoire si `DATABASE_URL` n'est
  pas définie. Vérifié par survie réelle à un redémarrage complet du
  processus (cf. `docs/DEVLOG.md`).
- **API HTTP** (§12) reliant le tout, avec table de correspondance
  écrans ↔ routes dans `crates/agoravote-api/src/routes.rs`.
- **Observabilité** : logs structurés (`tracing`), id de requête,
  détection des erreurs silencieuses (404/400 loggés, panics
  rattrapées) — voir `docs/LOGGING.md`.
- **Durcissement HTTP** : limite de taille de requête, timeout,
  arrêt propre sur SIGTERM — voir `docs/SECURITY.md`.
- **CI** (`.github/workflows/ci.yml`) : formatage, lints stricts,
  tests, build Docker — vérifiée aussi en local via `make check`.
- **Déploiement Docker** : `Dockerfile` multi-stage, `docker-compose.yml`
  (API + PostgreSQL + frontend), `install.sh`/`uninstall.sh` — voir
  `docs/DOCKER.md`.
- **Contrat d'API** (`docs/openapi.yaml`) : spécification OpenAPI 3.0
  validée par un outil dédié, source de vérité partagée avec le
  frontend (types TypeScript générés automatiquement).
- **Frontend** (`frontend/`) : interface React consommant l'API — cf.
  section dédiée plus haut et `frontend/README.md`.
- **48 tests automatisés**, tous verts (`cargo test --workspace`),
  dont 13 tests d'intégration du routeur HTTP (authentification,
  formulaire, anti double-vote, `GET /auth/me`), plus 8 tests d'intégration PostgreSQL
  (`cargo test -p agoravote-store -- --ignored`, nécessitent une base
  réelle).

## Ce qui n'est *pas* encore fait

Cf. cahier des charges §21 "Travaux restant à mener" — ce prototype
n'entame que la partie noyau/API. Restent, dans l'ordre où ils
bloqueraient probablement une mise en production :

1. **Permissions fines au-delà du rôle global** (§13) : aujourd'hui,
   `Organizer` peut administrer TOUTES les campagnes de son
   organisation, pas seulement les siennes ; pas de rattachement
   utilisateur ↔ campagne particulière.
2. **Flux d'invitation d'organisateurs** : l'inscription publique
   attribue directement le rôle `Organizer` (cf. commentaire de
   `register()` dans `routes.rs`) — un vrai contrôle d'accès à
   l'inscription reste à construire.
3. **Validation bulletin ↔ type de question** : rien n'empêche
   aujourd'hui de soumettre un bulletin `scores` sur une question
   `single_choice` ; la cohérence n'est vérifiée qu'au moment du
   dépouillement (le module de vote ignore ce qui ne le concerne pas),
   pas à la soumission.
4. **Journal d'audit** (§5, §13) : le type `AuditEvent` existe mais
   n'est pas encore produit par les handlers HTTP, ni persisté.
5. **Exports CSV/JSON** dédiés (§14) : aujourd'hui, seul le JSON brut
   de l'API est disponible.
6. **Interface web** (§10) : un frontend React existe (`frontend/`)
   mais reste partiel — construction de formulaire limitée à
   choix unique/multiple, pas de liste de campagnes côté serveur.
   Vérifié dans un vrai navigateur depuis l'itération 6 (cf.
   `docs/DEVLOG.md`).
7. **Méthodes de vote avancées** (Condorcet/Schulze, STV, jugement
   majoritaire — §15.1) : hors MVP, pas commencées.
8. **Modules WASM** (§6.1) : seul le chargement natif existe.
9. **Module d'identité Ğ1/Duniter** (`crates/agoravote-g1`) : vision
   posée dans l'addendum v0.3. Câblé de bout en bout depuis l'itération
   8 (`POST /auth/g1/challenge`/`verify`, écran `/connexion-g1`) pour
   la preuve de possession de clé (signature sr25519 réelle, jamais la
   phrase de 12 mots) — cf. `docs/DEVLOG.md`. La partie requête à la
   chaîne (`chain.rs`, vérification d'appartenance à la toile de
   confiance) compile mais reste **non vérifiée à l'exécution ni
   câblée à aucune route**, faute d'accès réseau à l'infrastructure
   Duniter dans tout environnement de développement utilisé jusqu'ici
   (cf. `crates/agoravote-g1/README.md`, `docs/G1_INTEGRATION.md` et
   `docs/SECURITY.md` §8 pour le détail complet et la marche à suivre).

## Licence

`AGPL-3.0-or-later` provisoire dans `Cargo.toml` — à confirmer par la
gouvernance du projet (cahier des charges §17).
