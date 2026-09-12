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

## Statut : compilé et testé pour `challenge`/`signature`, `chain` toujours non vérifié

**Mise à jour** (cf. `docs/DEVLOG.md`, itération 6) : compilé avec
succès dans un environnement à toolchain Rust à jour (rustc 1.94),
`docker compose`, accès réseau à `crates.io`/`static.rust-lang.org`.
`challenge.rs` et `signature.rs` sont désormais compilés **et
testés** (`cargo test -p agoravote-g1 --features chain-query`, 9
tests verts, dont un vecteur sr25519 connu — compte de développement
Substrate standard `//Alice`) — au même niveau d'exigence que le reste
du projet. Une seule correction d'API a été nécessaire : `verify` est
une fonction libre du module `subxt_signer::sr25519`, pas une méthode
de `PublicKey`, comme le fichier l'indiquait déjà comme hypothèse à
vérifier.

`chain.rs` compile également (API dynamique `subxt::dynamic`, non
typée contre un schéma précis) mais **reste non vérifié à
l'exécution** : l'infrastructure réseau Duniter/Ğ1 (`rpc.duniter.org`,
`g1-squid.axiom-team.fr`, un nœud `gdev`) reste bloquée par la liste
blanche réseau de cet environnement, testé à nouveau et confirmé
bloqué. Les noms de stockage (`IdentityIndexOf`, `Membership`) restent
donc des hypothèses non confirmées contre une métadonnée de nœud réel.

## Pour le rendre entièrement utilisable

1. ~~Compiler dans un environnement à toolchain Rust à jour~~ — fait
   (rustc 1.94, cf. ci-dessus).
2. **Confirmer les noms de stockage** contre un vrai nœud `gdev`
   (réseau de test — jamais `g1` en premier), depuis un environnement
   qui a accès réseau à l'infrastructure Duniter (celui-ci ne l'a
   toujours pas) : cf. `chain.rs` pour la liste précise de ce qui reste
   à confirmer (noms exacts des éléments de stockage des pallets
   `Identity` et `Membership`).
3. ~~Ajouter un test avec un vecteur sr25519 connu~~ — fait
   (`signature.rs`, vecteur `//Alice`).
4. **Réintégrer dans le workspace principal** une fois l'étape 2
   validée, en rajoutant `"crates/agoravote-g1"` à `members` dans le
   `Cargo.toml` racine — et en vérifiant que `cargo test --workspace`
   passe toujours entièrement à ce moment-là. Pas encore fait : ce
   crate reste volontairement hors du workspace principal tant que
   `chain.rs` (activé seulement par la feature `chain-query`, non
   utilisée par défaut) n'a pas été confronté à un nœud réel — même
   si, contrairement à l'itération précédente, l'inclure ne casserait
   plus la compilation du reste du workspace (toolchain suffisante
   désormais).

5. Seulement alors, câbler ce crate dans `agoravote-api` (nouvelles
   routes `/auth/g1/challenge`, `/auth/g1/verify` — cf. le sketch
   d'intégration dans `docs/G1_INTEGRATION.md`).
