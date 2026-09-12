# Observabilité — logs, erreurs, et erreurs silencieuses

Ce document répond à une question précise : *comment voir ce qui se
passe dans AgoraVote, avoir des messages de log utiles, et détecter
les erreurs — y compris celles qui ne "crashent" rien et passeraient
inaperçues ?*

## 1. La pile technique retenue

| Brique | Rôle |
|---|---|
| [`tracing`](https://docs.rs/tracing) | Émission de logs **structurés** (champs typés, pas juste du texte) et de **spans** (contexte qui englobe plusieurs lignes de log liées) |
| [`tracing-subscriber`](https://docs.rs/tracing-subscriber) | Met en forme et filtre ce que `tracing` émet ; lit la variable `RUST_LOG` |
| [`tower-http`](https://docs.rs/tower-http) (`TraceLayer`, `RequestId`, `CatchPanic`) | Branche `tracing` sur le cycle de vie HTTP sans dupliquer de code dans chaque handler |

**Pourquoi pas `println!` ou le crate `log` seul ?** `println!` ne
permet ni niveaux de sévérité, ni filtrage par module, ni structure
(un `println!("erreur: {e}")` est juste une chaîne de caractères, pas
une donnée qu'un outil de supervision peut interroger). Le crate `log`
donne les niveaux mais pas les *spans* : sans eux, dans un serveur qui
traite des requêtes concurrentes, il est impossible de savoir quelles
lignes de log appartiennent à la même requête. `tracing` résout les
trois problèmes à la fois — voir `main.rs::init_observability`.

## 2. Voir les logs : la variable `RUST_LOG`

```bash
# Défaut du projet si RUST_LOG n'est pas défini (cf. main.rs) :
# info partout, debug pour le code du projet et pour tower_http
cargo run -p agoravote-api

# Tout en debug (bruyant, utile pour traquer un bug précis)
RUST_LOG=debug cargo run -p agoravote-api

# Ne garder que les erreurs, sauf le détail HTTP en debug
RUST_LOG=error,tower_http=debug cargo run -p agoravote-api

# Cibler un seul module (ex: uniquement le moteur de vote)
RUST_LOG=agoravote_voting=trace cargo run -p agoravote-api
```

Sous Docker (`docker-compose.yml`), la même variable se règle via
`environment: RUST_LOG=...` — voir `docs/DOCKER.md`.

## 3. Ce que chaque requête HTTP produit dans les logs

Extrait réel (capturé pendant le développement, `RUST_LOG=debug`) pour
une requête sur une campagne inexistante :

```
DEBUG requete_http{id=a46a4d6d... methode=GET chemin=/campaigns/00000000... client_ip=127.0.0.1:49428}: started processing request
 WARN requete_http{id=a46a4d6d... ...}: agoravote_api::routes: crates/agoravote-api/src/routes.rs:391: ressource introuvable motif="campagne introuvable" statut=404
 INFO requete_http{id=a46a4d6d... ...}: agoravote_api::routes: requête traitée statut=404 latence_ms=0
```

Trois informations superposées automatiquement, sans que le handler
`get_campaign` n'ait rien fait d'explicite pour ça :

1. **Un span `requete_http`** avec un id unique, la méthode, le chemin,
   l'IP cliente — posé par `TraceLayer::make_span_with` (`routes.rs`).
2. **Une ligne `WARN`** avec le message métier précis et le
   fichier:ligne d'origine — posée par `not_found()`/`bad_request()`.
3. **Une ligne `INFO` finale** avec le statut HTTP et la latence —
   posée automatiquement par `TraceLayer::on_response`, pour TOUTE
   requête, succès ou échec.

L'id de requête (`a46a4d6d...`) est aussi renvoyé au client dans le
header `x-request-id` : si un utilisateur signale un bug, il suffit de
lui demander cet id (ou de le lire dans les outils réseau de son
navigateur) pour retrouver instantanément les trois lignes ci-dessus
dans les logs serveur, sans avoir à corréler par date/heure.

## 4. Catégories d'erreurs silencieuses, et comment ce projet les couvre

C'est le cœur de la question posée. Une "erreur silencieuse" est un
cas où le système fait quelque chose d'anormal sans que personne ne
le voie. Il en existe plusieurs familles, chacune avec un mécanisme
dédié :

| Famille d'erreur silencieuse | Sans protection | Mécanisme mis en place | Où dans le code |
|---|---|---|---|
| Une requête échoue (400/404) mais rien n'est loggé côté serveur | Le client seul sait qu'il y a eu une erreur ; le serveur n'en garde aucune trace | `not_found()`/`bad_request()` loggent systématiquement en `WARN` avant de construire la réponse | `routes.rs` |
| Une panique dans un handler coupe la connexion sans réponse ni log | Le client voit une connexion "reset" incompréhensible ; aucune trace exploitable | `CatchPanicLayer` : convertit en 500 + log `ERROR` | `routes.rs`, testé par `panique_dans_un_handler_devient_une_reponse_500` |
| Une panique **hors** d'un handler HTTP (tâche de fond future, job planifié) | Écrite sur stderr au format par défaut de Rust, invisible pour un système qui ne surveille que les logs `tracing` | Hook de panique global (`install_panic_hook`) qui journalise via `tracing::error!` avant de déléguer au comportement par défaut | `main.rs` |
| Un mutex empoisonné par une panique précédente fait paniquer TOUTES les requêtes suivantes en cascade | Le premier incident (déjà loggé grâce aux deux lignes ci-dessus) en déclenche silencieusement une infinité d'autres, masquées par la première | `lock_recover()` récupère les données même si le mutex est empoisonné, au lieu de propager la panique | `state.rs` |
| Un arrêt brutal du process (`docker stop`, `kill`) ne laisse aucune trace de fin propre | Impossible de distinguer un arrêt volontaire d'un crash en lisant seulement les logs | `shutdown_signal()` intercepte SIGTERM/Ctrl+C et logge le début ET la fin de l'arrêt (`"AgoraVote API arrêtée proprement"`) — son absence dans les logs est elle-même un signal de crash | `main.rs` |
| Un résultat de calcul silencieusement faux (pas une erreur système, mais un mauvais résultat qui ne plante rien) | Personne ne remarque qu'une méthode de vote calcule mal | Traité différemment : par les **tests unitaires contre des valeurs connues** (§8.2, §8.3), pas par les logs — les logs détectent qu'*un incident s'est produit*, les tests détectent qu'*un calcul est faux* alors même que rien n'a "planté" | `agoravote-voting`, `agoravote-stats`, tous les modules `#[cfg(test)]` |
| Un `Result` ignoré (l'erreur existe mais n'est jamais lue) | La fonction appelante ne sait jamais qu'une opération a échoué | Prévenu structurellement par Clippy : `#[warn(unused_must_use)]` est actif par défaut sur tout `Result` retourné par une fonction ; notre CI fait tourner `clippy -D warnings`, qui transforme ce warning en échec de build — voir `docs/DEVELOPMENT_WORKFLOW.md` | `.github/workflows/ci.yml` |

## 5. Reproduire la preuve par vous-même

Chacun des mécanismes ci-dessus est démontré par un test qui
échouerait s'il cessait de fonctionner — pas seulement décrit en
prose :

```bash
cargo test -p agoravote-api -- --nocapture
```

- `request_id_est_propage_dans_la_reponse` — vérifie que chaque
  réponse porte un `x-request-id`. Ce test a d'ailleurs **détecté un
  vrai bug** pendant le développement de cette fonctionnalité (un
  mauvais ordre de couches faisait que l'id n'était jamais propagé,
  silencieusement) — voir `docs/DEVLOG.md` pour le récit complet.
- `panique_dans_un_handler_devient_une_reponse_500` — provoque une
  vraie panique dans un handler de test et vérifie qu'elle devient un
  500 plutôt qu'une connexion coupée.
- `campagne_introuvable_renvoie_404` / `health_repond_ok` — vérifient
  le comportement nominal du routeur.

Pour voir les logs en conditions réelles :

```bash
cargo build -p agoravote-api
RUST_LOG=debug ./target/debug/agoravote-api &
curl http://localhost:3000/campaigns/00000000-0000-0000-0000-000000000000
# observer la ligne WARN "ressource introuvable" dans le terminal du serveur
```

## 6. Niveaux de log : convention du projet

| Niveau | Quand l'utiliser | Exemple dans ce projet |
|---|---|---|
| `error` | Le serveur a un problème qui lui est propre (bug, panique, dépendance indisponible) | `CatchPanicLayer`, `TraceLayer::on_failure`, hook de panique |
| `warn` | La requête échoue pour une raison légitime côté client (donnée invalide, ressource absente), mais ça mérite d'être visible en cas de pic anormal | `not_found()`, `bad_request()` |
| `info` | Événements normaux qui structurent la compréhension du système (démarrage, arrêt, requête traitée) | `main.rs`, `TraceLayer::on_response` |
| `debug` | Détail utile en développement ou pour investiguer un incident précis, trop verbeux pour tourner en permanence en production | Détail des requêtes HTTP (`tower_http=debug`) |
| `trace` | Détail fin d'un module précis, activé ponctuellement (`RUST_LOG=agoravote_voting=trace`) | Non utilisé pour l'instant ; réservé pour un futur diagnostic fin d'une méthode de vote |

## 7. Ce qui manque encore (prochaines étapes d'observabilité)

- **Le journal d'audit fonctionnel** (`agoravote_core::AuditEvent`,
  §5/§13 du cahier des charges) est un concept différent des logs
  techniques décrits ici : c'est un historique métier ("qui a publié
  quelle campagne, quand"), destiné à l'écran "15. Audit" et à un
  usage réglementaire/légal, pas au diagnostic technique. Il existe
  dans le modèle mais n'est pas encore émis par les handlers — cf.
  `docs/ARCHITECTURE.md`.
- **Export des métriques** (nombre de requêtes, latence agrégée,
  erreurs par minute) vers un système comme Prometheus : aucun
  mécanisme pour l'instant, seulement des logs. Candidat naturel :
  `axum-prometheus` ou une exposition manuelle via `tower_http::metrics`.
- **Logs au format JSON** pour une ingestion facile par un agrégateur
  (Loki, ELK...) : le format actuel (`tracing_subscriber::fmt`) est
  lisible par un humain dans un terminal, pas optimisé pour une
  machine. Basculer vers `tracing_subscriber::fmt().json()` est une
  modification d'une ligne dans `main.rs` le jour où c'est nécessaire.
