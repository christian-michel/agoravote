# agoravote-g1

Module d'identité **optionnel** pour AgoraVote : preuve de possession
d'un compte Ğ1v2 (signature) et vérification d'adhésion à la toile de
confiance (requête à la chaîne). Cf. l'addendum v0.3 du cahier des
charges (« Identité décentralisée et trajectoire Web3 ») et
`docs/G1_INTEGRATION.md` à la racine du dépôt pour la documentation
complète, ainsi que `docs/SECURITY.md` §8 pour les choix de sécurité.

## Membre du workspace principal depuis l'itération 8

Ce crate a longtemps eu sa propre section `[workspace]` (regardez
l'historique git de `Cargo.toml`, ici et à la racine du dépôt) : sa
chaîne de dépendances cryptographiques (`subxt-signer`, `subxt`) ne
compilait pas avec la toolchain `rustc 1.75` utilisée à l'origine pour
développer le reste du projet, et un workspace Cargo résout un seul
`Cargo.lock` pour tous ses membres — l'inclure aurait alors cassé la
compilation de tout le reste (`cargo test --workspace`, la CI, `make
check`). Avec une toolchain à jour (rustc 1.94, cf. `docs/DEVLOG.md`
itération 6), cette contrainte a disparu : `crates/agoravote-g1` est
réintégré à `members` dans le `Cargo.toml` racine depuis l'itération 8
et partage désormais le même `Cargo.lock` que le reste du projet.
Cette réintégration était nécessaire, pas seulement possible :
`agoravote-api` a besoin d'en dépendre par chemin pour câbler les
routes `/auth/g1/challenge`/`/auth/g1/verify`, ce que Cargo refuse tant
qu'un crate déclare sa propre section `[workspace]` séparée
("multiple workspace roots found in the same workspace").

## Statut : `challenge`/`signature` câblés en production, `chain` toujours non vérifié

`challenge.rs` et `signature.rs` sont compilés, testés (`cargo test -p
agoravote-g1 --features chain-query`, 13 tests verts, dont un vecteur
**ed25519** — cf. ci-dessous pour pourquoi ed25519 et pas sr25519)
**et utilisés en production** : `agoravote-api` dépend de la feature
`signature-verification` (cf. son `Cargo.toml`) pour
`POST /auth/g1/challenge` et `POST /auth/g1/verify` (cf.
`docs/G1_INTEGRATION.md` §4 et `docs/DEVLOG.md` itérations 8-9). Le
protocole défi/signature prouve la possession de clé sans jamais
transmettre la phrase de 12 mots, avec une fraîcheur de défi vérifiable
sans état serveur (`Challenge::verify_freshness`, horodatage embarqué
dans le texte signé).

### ⚠️ Correctif de schéma cryptographique (itération 9) : ed25519, pas sr25519

Une version antérieure de ce crate (itération 8) vérifiait des
signatures **sr25519** via `subxt-signer`, en présumant sans
vérification que c'était le schéma utilisé par les comptes Ğ1v2. Des
recherches web ont montré que c'est probablement faux : la communauté
Duniter a débattu explicitement de ce choix (fil de forum "Use ed25519
vs sr25519 for v2s clients") et les comptes de développement/production
Duniter v2s semblent alignés sur **ed25519** — le même schéma que la
Ğ1 historique (v1), pour l'interopérabilité avec les outils tiers,
sr25519 étant spécifique à l'écosystème Polkadot sans bénéfice propre
pour Duniter. `signature.rs` a donc été réécrit pour vérifier des
signatures ed25519 via `ed25519-zebra` (`subxt-signer` ne propose de
toute façon aucun module `ed25519`, seulement `sr25519`/`ecdsa`,
vérifié en lisant son code source). Cette correction s'appuie sur des
recherches web, pas sur une lecture directe de la documentation
officielle Duniter (`forum.duniter.org`/`git.duniter.org` restent
bloqués par la liste blanche réseau de cet environnement, comme
`chain.rs`) — **à reconfirmer avant toute mise en production**, cf.
`docs/SECURITY.md` §8.

`chain.rs` compile également (API dynamique `subxt::dynamic`, non
typée contre un schéma précis, feature `chain-query`) mais **reste non
vérifié à l'exécution ni câblé à aucune route** : l'infrastructure
réseau Duniter/Ğ1 (`rpc.duniter.org`, `g1-squid.axiom-team.fr`, un
nœud `gdev`) reste bloquée par la liste blanche réseau de tout
environnement de développement utilisé jusqu'ici, testé à nouveau et
confirmé bloqué en itération 8. Les noms de stockage
(`IdentityIndexOf`, `Membership`) restent donc des hypothèses non
confirmées contre une métadonnée de nœud réel — cf. `docs/SECURITY.md`
§8 pour l'explication complète de ce que `POST /auth/g1/verify` prouve
(possession de clé) et ne prouve PAS (appartenance à la toile de
confiance) tant que `chain-query` n'est pas câblé.

## Pour aller plus loin

1. ~~Compiler dans un environnement à toolchain Rust à jour~~ — fait
   (rustc 1.94, cf. `docs/DEVLOG.md` itération 6).
2. **Confirmer les noms de stockage** contre un vrai nœud `gdev`
   (réseau de test — jamais `g1` en premier), depuis un environnement
   qui a accès réseau à l'infrastructure Duniter (celui-ci ne l'a
   toujours pas) : cf. `chain.rs` pour la liste précise de ce qui reste
   à confirmer (noms exacts des éléments de stockage des pallets
   `Identity` et `Membership`).
3. ~~Ajouter un test avec un vecteur cryptographique connu~~ — fait
   (`signature.rs`, vecteur ed25519 depuis l'itération 9 — cf.
   ci-dessus pour le correctif sr25519 -> ed25519).
4. ~~Réintégrer dans le workspace principal~~ — fait, cf. ci-dessus
   (itération 8).
5. ~~Câbler ce crate dans `agoravote-api`~~ — fait pour
   `signature-verification` (itération 8, cf.
   `docs/G1_INTEGRATION.md` §4). Reste à faire une fois l'étape 2
   validée : câbler `chain-query` (`chain::check_membership`) comme
   vérification optionnelle supplémentaire, par campagne, sur
   `/auth/g1/verify` — sans jamais la présenter comme déjà active tant
   qu'elle ne l'est pas.
