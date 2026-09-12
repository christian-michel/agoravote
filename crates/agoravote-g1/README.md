# agoravote-g1

Module d'identité **optionnel** pour AgoraVote : preuve de possession
d'un compte Ğ1v2 (signature) et vérification d'adhésion à la toile de
confiance (requête à la chaîne). Cf. l'addendum v0.3 du cahier des
charges (« Identité décentralisée et trajectoire Web3 ») et
`docs/G1_INTEGRATION.md` à la racine du dépôt pour la documentation
complète.

## ⚠️ Pourquoi ce crate est hors du workspace principal

Regardez `Cargo.toml` à la racine du dépôt : `crates/agoravote-g1`
**n'est pas** dans la liste `members`. C'est volontaire, pas un oubli.

Sa chaîne de dépendances (cryptographie Substrate : `subxt-signer`,
`subxt`) ne compile pas avec la toolchain `rustc 1.75` utilisée pour
développer le reste de ce projet (voir le détail des blocages dans
`Cargo.toml` de ce crate). Un workspace Cargo résout **un seul**
`Cargo.lock` pour tous ses membres : si ce crate y était inclus,
**tout le reste du projet cesserait de compiler** — `cargo test
--workspace`, la CI, `make check`, tout. Ça s'est produit une fois
pendant le développement (cf. `docs/DEVLOG.md`) et a été corrigé en
sortant ce crate du workspace.

Techniquement, `crates/agoravote-g1` est donc son propre petit
workspace à un seul membre (cf. la section `[workspace]` vide dans son
`Cargo.toml`), avec son propre `Cargo.lock`, indépendant de celui du
projet principal.

## Statut : code écrit, non compilé, non testé

**C'est la seule exception dans tout ce projet.** Partout ailleurs
(`agoravote-core`, `agoravote-voting`, `agoravote-stats`,
`agoravote-store`, `agoravote-auth`, `agoravote-api`), chaque ligne
livrée a été compilée et testée avant d'être présentée comme
fonctionnelle — pour `agoravote-store`, contre un vrai PostgreSQL
local. Ici, ça n'a pas été possible : la toolchain locale ne compile
pas les dépendances crypto (cf. ci-dessus), et l'environnement de
développement n'a de toute façon pas accès réseau à l'infrastructure
Duniter/Ğ1 (testé, bloqué par la liste blanche réseau du bac à sable —
cf. la conversation qui a précédé ce crate).

Le code a été écrit avec le plus grand soin, en s'appuyant sur la
documentation officielle et le code source du runtime Duniter v2
consultés le 10 septembre 2026 — mais reste non vérifié. Chaque
fichier documente précisément ce qui est confirmé et ce qui ne l'est
pas.

## Pour le rendre utilisable

1. **Compiler dans un environnement à toolchain Rust à jour** (pas
   celui-ci) :
   ```bash
   cd crates/agoravote-g1
   cargo build --features chain-query
   ```
   corriger les éventuelles erreurs d'API (`subxt`/`subxt-signer`
   évoluent ; les noms de méthodes utilisés dans `signature.rs` et
   `chain.rs` n'ont pas pu être vérifiés).

2. **Confirmer les noms de stockage** contre un vrai nœud `gdev`
   (réseau de test — jamais `g1` en premier) : cf. `chain.rs` pour la
   liste précise de ce qui reste à confirmer (noms exacts des éléments
   de stockage des pallets `Identity` et `Membership`).

3. **Ajouter un test avec un vecteur sr25519 connu** dans
   `signature.rs` (marqué comme manquant dans le fichier).

4. **Réintégrer dans le workspace principal** une fois 1-3 validés, en
   rajoutant `"crates/agoravote-g1"` à `members` dans le `Cargo.toml`
   racine — et en vérifiant que `cargo test --workspace` passe
   toujours entièrement à ce moment-là.

5. Seulement alors, câbler ce crate dans `agoravote-api` (nouvelles
   routes `/auth/g1/challenge`, `/auth/g1/verify` — cf. le sketch
   d'intégration dans `docs/G1_INTEGRATION.md`).
