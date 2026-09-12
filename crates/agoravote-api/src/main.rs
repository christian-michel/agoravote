//! # agoravote-api
//!
//! Serveur HTTP de démonstration exposant le noyau AgoraVote
//! (`agoravote-core` + `agoravote-voting` + `agoravote-stats`) —
//! cf. cahier des charges §12 « Architecture technique cible ».
//!
//! **Statut : prototype de câblage, pas encore l'API cible.** Son
//! seul but est de démontrer, de bout en bout, que le modèle §5, le
//! contrat de module §6 et les méthodes de vote §8.2 fonctionnent
//! ensemble : créer une campagne → créer un formulaire → publier →
//! voter → dépouiller → consulter le résultat. Ce qui manque
//! explicitement pour une vraie v1 (cf. §21, "travaux restants") :
//! authentification/permissions (§13), persistance PostgreSQL (§12.2),
//! validation fine des bulletins par rapport au type de question,
//! export CSV/JSON (§14), journal d'audit (§5, §13), et bien sûr
//! l'interface web elle-même (§10).
//!
//! ## Lancer le serveur
//!
//! ```bash
//! cargo run -p agoravote-api
//! # puis, dans un autre terminal :
//! curl http://localhost:3000/health
//! ```
//!
//! Voir `docs/API_WALKTHROUGH.md` à la racine du dépôt pour un
//! parcours complet (créer une campagne, voter, dépouiller), et
//! `docs/LOGGING.md` pour le détail de la stratégie d'observabilité
//! mise en place dans ce fichier et dans `routes.rs`.

mod auth;
mod dto;
mod routes;
mod state;

use state::AppState;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    init_observability();
    install_panic_hook();

    let state = build_state().await;
    let app = routes::build_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("impossible de démarrer le serveur : port 3000 déjà utilisé ?");

    tracing::info!(addr = "0.0.0.0:3000", "AgoraVote API démarrée");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("erreur fatale du serveur HTTP");

    // Ce point n'est atteint qu'après un arrêt propre (cf.
    // `shutdown_signal`) : s'il ne s'affiche jamais dans les logs, c'est
    // le signe que le serveur s'est arrêté brutalement (SIGKILL, panic
    // non rattrapée en dehors d'un handler HTTP) plutôt que proprement.
    tracing::info!("AgoraVote API arrêtée proprement");
}

/// Choisit et initialise le backend de persistance au démarrage —
/// cf. `agoravote-store` pour le détail des deux backends.
///
/// La règle est volontairement simple et explicite dans les logs : la
/// présence de `DATABASE_URL` décide tout. Pas de détection implicite
/// ("essayer Postgres, retomber sur la mémoire si ça échoue") — un
/// échec de connexion à une base explicitement configurée doit être
/// une erreur fatale et bruyante au démarrage (`.expect(...)`), pas un
/// repli silencieux vers un mode où les données seraient perdues sans
/// avertissement. C'est le même principe que la détection des
/// "erreurs silencieuses" documentée dans `docs/LOGGING.md` : mieux
/// vaut un démarrage qui échoue bruyamment qu'un serveur qui démarre
/// dans un mode différent de celui demandé sans le signaler clairement.
async fn build_state() -> AppState {
    match std::env::var("DATABASE_URL") {
        Ok(database_url) => {
            tracing::info!("DATABASE_URL détectée, connexion au backend PostgreSQL...");
            AppState::new_postgres(&database_url)
                .await
                .expect("connexion/migration PostgreSQL — voir DATABASE_URL et docs/DOCKER.md")
        }
        Err(_) => {
            tracing::warn!(
                "DATABASE_URL absente : backend en mémoire utilisé (§1.1 \"auto-hébergement / \
                 exécution locale\") — TOUTES LES DONNÉES SERONT PERDUES à l'arrêt du processus. \
                 Voir docs/DOCKER.md pour activer la persistance PostgreSQL."
            );
            AppState::new_memory()
        }
    }
}

/// Configure la journalisation structurée de l'application.
///
/// ## Pourquoi `tracing` plutôt que `println!`/`log` ?
///
/// `println!` ne permet ni niveaux de sévérité, ni filtrage par
/// module, ni corrélation entre plusieurs lignes d'une même requête.
/// `tracing` résout les trois : chaque requête HTTP obtient un *span*
/// (cf. `routes.rs`, `TraceLayer`) auquel toutes les lignes de log
/// émises pendant son traitement sont automatiquement rattachées, avec
/// un identifiant de requête commun — utile pour retrouver, dans un
/// flux de logs de production sous forte charge, toutes les lignes
/// relatives à UNE requête précise qui a échoué.
///
/// ## Contrôler la verbosité : la variable `RUST_LOG`
///
/// Le niveau par défaut (si `RUST_LOG` n'est pas défini) est choisi
/// pour être utile en développement sans être bruyant : `info` pour
/// tout, `debug` pour le code de ce projet (`agoravote_api`,
/// `agoravote_core`, `agoravote_voting`, `agoravote_stats`) et pour les
/// requêtes HTTP (`tower_http`). Exemples de réglages plus précis :
///
/// ```bash
/// # Tout en debug (très verbeux, utile pour un bug précis)
/// RUST_LOG=debug cargo run -p agoravote-api
///
/// # Ne voir que les erreurs, sauf le détail des requêtes HTTP
/// RUST_LOG=error,tower_http=debug cargo run -p agoravote-api
///
/// # Cibler un seul module (ex: déboguer uniquement le dépouillement)
/// RUST_LOG=agoravote_voting=trace cargo run -p agoravote-api
/// ```
fn init_observability() {
    let default_filter = "info,agoravote_api=debug,agoravote_core=debug,\
                           agoravote_voting=debug,agoravote_stats=debug,agoravote_store=debug,tower_http=debug";
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                // Inclut fichier + ligne d'origine du log : précieux
                // pour retrouver l'endroit exact d'une erreur sans
                // deviner à partir du seul message.
                .with_file(true)
                .with_line_number(true)
                .with_target(true),
        )
        .init();
}

/// Installe un gestionnaire de panique global qui journalise via
/// `tracing` (message, fichier, ligne) avant de laisser le
/// comportement par défaut de Rust s'exécuter (impression sur stderr
/// + avortement du thread courant).
///
/// ## Pourquoi c'est nécessaire pour détecter les "erreurs silencieuses"
///
/// Sans ce hook, une panique dans du code appelé *en dehors* d'un
/// handler HTTP (par exemple une future tâche de fond, un job planifié
/// qui clôturerait des campagnes automatiquement) n'apparaît que sur
/// stderr, dans le format par défaut de Rust, sans passer par notre
/// pipeline de logs structurés — donc invisible pour un système de
/// supervision qui ne surveille que les logs `tracing` (fichier,
/// agrégateur centralisé...). Ce hook garantit qu'AUCUNE panique, où
/// qu'elle survienne dans le processus, n'échappe aux logs applicatifs.
///
/// Pour les panics qui surviennent *dans* un handler HTTP, voir aussi
/// `CatchPanicLayer` dans `routes.rs` : les deux mécanismes sont
/// complémentaires (celui-ci journalise et couvre tout le processus ;
/// `CatchPanicLayer` empêche en plus la panique de faire tomber la
/// connexion HTTP sans réponse).
fn install_panic_hook() {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "emplacement inconnu".to_string());

        tracing::error!(
            panic.location = %location,
            panic.message = %panic_info,
            "panique non rattrapée dans le processus"
        );

        previous_hook(panic_info);
    }));
}

/// Attend un signal d'arrêt (Ctrl+C, ou SIGTERM envoyé par `docker stop`)
/// pour permettre à `axum::serve` de terminer proprement les requêtes en
/// cours avant de couper le serveur.
///
/// Sans ceci, `docker stop` envoie SIGTERM, le processus l'ignore (aucun
/// gestionnaire par défaut pour un serveur Tokio), et Docker attend le
/// délai de grâce complet (10s par défaut) avant d'envoyer SIGKILL — un
/// arrêt "sale" à chaque déploiement/redémarrage de conteneur.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("impossible d'installer le gestionnaire Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("impossible d'installer le gestionnaire SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("Ctrl+C reçu, arrêt en cours..."),
        () = terminate => tracing::info!("SIGTERM reçu, arrêt en cours..."),
    }
}
