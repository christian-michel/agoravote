# Process de travail — débogage, évolution, documentation

Ce document décrit **comment travailler sur ce projet au quotidien** :
comment déboguer un problème, comment faire évoluer le code sans le
dégrader, et comment la documentation/les commentaires s'intègrent à
ce processus plutôt que d'être une corvée séparée.

## 1. Boucle de développement locale

```bash
make check   # fmt --check + clippy -D warnings + test — tout ce que la CI vérifie
make run     # démarre l'API avec des logs verbeux (RUST_LOG=debug)
```

`make check` avant CHAQUE commit. Ce n'est pas une formalité : les
trois vérifications qu'il enchaîne (formatage, lints, tests) sont
précisément les trois filets qui, dans ce projet, ont déjà rattrapé
des problèmes réels pendant le développement — voir `docs/DEVLOG.md`
pour l'exemple concret d'un bug d'ordre de middleware détecté par un
test qui aurait échoué silencieusement en CI sans jamais être vu en
local si `make check` n'avait pas été lancé avant de committer.

## 2. Déboguer un problème : la démarche

1. **Reproduire avec des logs verbeux.** `RUST_LOG=debug` (ou
   `RUST_LOG=agoravote_voting=trace` pour cibler un seul crate) et
   rejouer la requête qui pose problème avec `curl`. Voir
   `docs/LOGGING.md` pour le détail complet de ce que chaque requête
   produit comme logs et pourquoi.
2. **Chercher l'id de requête** (`x-request-id` dans la réponse) et
   filtrer les logs sur cet id pour isoler exactement le
   traitement de CETTE requête, même sous forte charge concurrente.
3. **Si le comportement est "faux" mais que rien ne plante ni ne
   logge d'erreur** (un résultat de vote incorrect, par exemple) : ce
   n'est plus un problème d'observabilité mais de correction — écrire
   un test qui reproduit le cas avec des données minimales (cf. §3.3),
   le voir échouer, puis corriger. Ne jamais corriger "à l'œil" sans
   d'abord avoir un test rouge : sans lui, rien ne garantit que le même
   bug ne revienne pas silencieusement plus tard.
4. **Si le comportement plante (panique)** : `CatchPanicLayer` (pour
   une requête HTTP) ou le hook de panique global (`main.rs`, pour le
   reste) garantissent qu'il y a TOUJOURS une ligne `ERROR` dans les
   logs avec le fichier et la ligne d'origine — partir de là.

## 3. Faire évoluer le projet sans le dégrader

### 3.1 Où va le nouveau code ?

Avant d'écrire une ligne, se poser la question : *quelle couche est
concernée ?* (cf. `docs/ARCHITECTURE.md`) — modèle de données pur →
`agoravote-core` ; algorithme de dépouillement → `agoravote-voting` ;
calcul statistique agnostique du domaine → `agoravote-stats` ; câblage
HTTP → `agoravote-api`. Si la réponse n'est pas évidente, c'est
souvent le signe qu'il faut d'abord clarifier la couche
correspondante dans le cahier des charges avant de coder.

### 3.2 Convention de documentation (à respecter pour tout nouveau fichier)

Chaque module du projet suit la même forme, visible dans n'importe
quel fichier existant :

```rust
//! Une ou deux phrases : QUEL rôle joue ce fichier.
//!
//! Renvoi explicite à la section du cahier des charges qui le
//! justifie (« cf. cahier des charges §X »), et, si un choix de
//! conception n'est pas évident, la raison de ce choix — pas
//! seulement CE qui est fait, mais POURQUOI.
```

Puis, pour chaque décision non triviale dans le corps du fichier
(pourquoi cette formule plutôt qu'une autre, pourquoi ce cas limite
est géré de cette façon), un commentaire au fil du code plutôt qu'un
gros bloc en tête de fichier — cf. `agoravote-stats/src/descriptive.rs`
pour un exemple dense (choix de la variance d'échantillon, méthode de
calcul des quartiles). **Un commentaire qui répète ce que le code dit
déjà ne sert à rien ; un commentaire qui explique pourquoi une
alternative plus évidente a été rejetée en vaut dix.**

### 3.3 Convention de tests

- Tout module de calcul (méthode de vote, statistique) est testé
  contre des **valeurs connues**, calculées à la main ou tirées d'une
  référence externe — jamais seulement "le code ne panique pas".
- Tout comportement d'erreur (refus de publier sans formulaire, quorum
  non atteint...) a son propre test qui vérifie le message/le type
  d'erreur exact, pas seulement `is_err()`.
- Tout mécanisme d'infrastructure (routage, middleware) qui pourrait
  échouer silencieusement a un test d'intégration qui le prouve — cf.
  `agoravote-api/src/routes.rs::tests` et `docs/LOGGING.md` §5.
- Nommage des tests en français, descriptif de ce qui est vérifié
  (`refuse_de_publier_sans_formulaire`, pas `test_publish_2`) : le nom
  du test doit suffire à comprendre ce qui a cassé dans un rapport de
  CI, sans avoir à ouvrir le fichier.

### 3.4 Avant d'ajouter une dépendance

1. Vérifier qu'elle est vraiment nécessaire (le workspace reste
   volontairement à peu de dépendances directes).
2. Vérifier son statut dans la base [RustSec](https://rustsec.org/)
   (`cargo audit` si la toolchain le permet, sinon consultation
   manuelle — cf. `docs/SECURITY.md` pour la méthode utilisée quand
   `cargo-audit` n'a pas pu tourner).
3. Documenter le choix si une contrainte d'environnement impose une
   version particulière (cf. la note sur `uuid` dans `Cargo.toml`).

## 4. Pourquoi `-D warnings` sur clippy (et pas seulement les erreurs)

Un choix assumé : la CI et `make check` échouent sur le moindre
warning clippy, pas seulement sur les lints les plus graves. Deux
warnings corrigés pendant le développement de ce projet
(`partial_cmp().unwrap()` pouvant paniquer sur `NaN`,
`.lock().unwrap()` propageant l'empoisonnement d'un mutex à toute
l'API) n'étaient QUE des warnings, pas des erreurs de compilation —
sans `-D warnings`, ils seraient passés inaperçus indéfiniment. Voir
`docs/SECURITY.md` pour le détail de cette revue.

## 5. Revue de code (quand le projet aura plusieurs contributeurs)

Checklist minimale avant de fusionner une pull request :

- [ ] `make check` passe (la CI le revérifie de toute façon, mais
      autant ne pas lui faire perdre de temps).
- [ ] Le nouveau code a des tests qui échoueraient si le comportement
      changeait par erreur.
- [ ] Les commentaires expliquent des décisions, pas des évidences.
- [ ] Si le changement touche à une section du cahier des charges,
      la référence (`§X`) est mise à jour dans les commentaires
      concernés.
- [ ] Si le changement affecte l'API HTTP, `docs/ARCHITECTURE.md`
      (table routes ↔ écrans) est mis à jour.

## 6. Documents à connaître

| Document | Contenu |
|---|---|
| `README.md` | Vue d'ensemble, démarrage rapide, périmètre fait/pas fait |
| `docs/ARCHITECTURE.md` | Correspondance fichier ↔ section du cahier des charges |
| `docs/LOGGING.md` | Stratégie de logs et détection des erreurs silencieuses (ce document la complète côté process) |
| `docs/SECURITY.md` | Revue de sécurité : dépendances, durcissement HTTP, limites connues |
| `docs/DOCKER.md` | Déploiement via Docker (Zorin OS / toute distribution Linux) |
| `docs/DEVLOG.md` | Journal des décisions prises et pourquoi, itération par itération |
