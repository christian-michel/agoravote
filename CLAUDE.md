# CLAUDE.md

Instructions de contexte pour Claude Code travaillant sur ce dépôt.
Lu automatiquement en début de session — pas besoin de le résumer à
l'utilisateur, mais son contenu doit guider chaque action.

## Ce qu'est ce projet

AgoraVote : plateforme libre de sondage, consultation et vote,
modulaire (méthodes de vote en modules interchangeables), auto-
hébergeable. Cf. `Cahier_des_charges_AgoraVote_v0_2.docx` et
`Addendum_AgoraVote_v0_3_identite_web3.docx` à la racine pour la
vision produit complète — ce dépôt en est l'implémentation naissante,
pas encore le produit fini.

**Avant toute action, lire dans l'ordre** :
1. `README.md` — état actuel, ce qui est fait/pas fait, démarrage rapide
2. `docs/ROADMAP.md` — priorités actuelles, dans l'ordre
3. `docs/ARCHITECTURE.md` — correspondance fichier ↔ section du cahier des charges
4. `docs/DEVLOG.md` — décisions déjà prises et pourquoi (pour ne pas re-débattre ce qui a déjà été tranché, ni répéter une erreur déjà corrigée)

Ce dépôt contient une configuration GitHub Codespaces
(`.devcontainer/devcontainer.json`, cf. `docs/CODESPACES.md`) : si tu
t'exécutes dans un Codespace ouvert depuis ce dépôt, Rust, Node et
Docker sont déjà prêts — inutile de les (ré)installer.

## Contexte d'origine important

Ce projet a été développé jusqu'ici dans une conversation Claude.ai
(interface de chat), dans un bac à sable avec des limites fortes :
toolchain Rust ancienne (rustc 1.75, sans `rustup`), aucun accès
réseau à l'infrastructure Ğ1/Duniter, aucun démon Docker, aucun
navigateur. Ces limites ont façonné certaines décisions (versions de
dépendances épinglées, `agoravote-g1` hors du workspace principal,
frontend jamais vérifié visuellement) — **elles ne s'appliquent
probablement plus dans cet environnement**. La toute première chose à
faire est de vérifier ce qui devient possible :

```bash
rustc --version   # si >= 1.80, réévaluer les épinglages de crates/agoravote-store/Cargo.toml
docker --version && docker compose version
cd crates/agoravote-g1 && cargo test --features signature-verification
```

Si `agoravote-g1` compile ici, cf. `crates/agoravote-g1/README.md`
pour la marche à suivre complète (réintégration au workspace
principal, confirmation des noms de stockage contre un nœud `gdev`
réel).

## Commandes essentielles

```bash
make check              # fmt --check + clippy -D warnings + tous les tests Rust — à lancer avant TOUT commit
cargo test -p agoravote-store -- --ignored   # tests PostgreSQL réels (nécessite DATABASE_URL)
cd frontend && npx tsc -b && npx oxlint && npm run build   # vérif frontend
docker compose up -d    # API + PostgreSQL + frontend — http://localhost:4080
```

## Conventions non négociables de ce projet

Ce ne sont pas des préférences de style, ce sont des règles déjà
appliquées partout dans le code existant — les respecter, pas les
redécouvrir :

- **Jamais de code non testé présenté comme fonctionnel.** Si quelque
  chose n'a pas pu être vérifié (pas d'accès réseau, dépendance qui ne
  compile pas...), le dire explicitement dans le code ET dans la doc
  — cf. `crates/agoravote-g1/` pour l'exemple de comment documenter un
  statut "écrit mais non vérifié" honnêtement.
- **Chaque commentaire de code renvoie à une section du cahier des
  charges** quand la décision en découle (`// cf. cahier des charges
  §X`), et explique le POURQUOI d'un choix non évident, pas ce que le
  code fait déjà lisiblement.
- **`docs/openapi.yaml` est la source de vérité du contrat d'API.**
  Toute modification de route côté `agoravote-api` doit s'y refléter
  IMMÉDIATEMENT, suivie de `cd frontend && npm run gen:api`. Un champ
  de réponse toujours présent doit être dans `required:` — ne pas
  laisser le frontend deviner via des `!`/vérifications défensives ce
  que l'API garantit déjà.
- **Aucune erreur silencieuse.** Tout chemin d'erreur HTTP (400/401/
  403/404/500) logue côté serveur avant de répondre — cf.
  `docs/LOGGING.md`. Toute nouvelle route suit ce patron.
- **Tests d'intégration réels, pas de mocks pour le cœur métier.** Le
  backend est testé contre un vrai PostgreSQL (cf.
  `crates/agoravote-store/tests/`), pas une base simulée.
- **Séparation des crates stricte** (cf. `docs/ARCHITECTURE.md`) :
  `agoravote-core` ne dépend de rien de métier ; `agoravote-voting`/
  `agoravote-stats`/`agoravote-auth` ne font aucune I/O ; seul
  `agoravote-store` touche à la base ; seul `agoravote-api` fait de
  l'HTTP. Ne pas casser cette frontière pour une commodité ponctuelle.
- **Jamais transmettre un secret utilisateur au serveur** (mot de
  passe en clair au-delà du hachage immédiat, phrase de 12 mots Ğ1,
  jeton de session en clair au repos) — cf. `docs/SECURITY.md` §7 pour
  le détail de chaque mécanisme déjà en place.

## Où documenter ce qui change

| Type de changement | Où le documenter |
|---|---|
| Nouvelle route API | `docs/openapi.yaml` + régénérer les types frontend |
| Décision d'architecture non triviale | `docs/DEVLOG.md`, nouvelle itération numérotée |
| Nouveau crate ou fichier | `docs/ARCHITECTURE.md`, table correspondante |
| Limite de sécurité connue | `docs/SECURITY.md` |
| Item de roadmap complété ou ajouté | `docs/ROADMAP.md` |

## Avant de proposer une pull request / un commit

1. `make check` passe.
2. Si `agoravote-api` ou `agoravote-store` a changé : tests
   d'intégration PostgreSQL relancés (`DATABASE_URL=... cargo test -p
   agoravote-store -- --ignored`).
3. Si l'API a changé : `docs/openapi.yaml` à jour, types frontend
   régénérés, `cd frontend && npx tsc -b` passe.
4. `docs/DEVLOG.md` mis à jour si la décision n'est pas triviale.
5. `docs/ROADMAP.md` mis à jour si un item a été complété.
