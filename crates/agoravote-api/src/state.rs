//! État applicatif partagé de l'API.
//!
//! Cf. cahier des charges §12 : l'API expose le noyau Rust au reste
//! du système. La persistance elle-même (mémoire ou PostgreSQL) est
//! déléguée au crate [`agoravote_store`] — ce fichier ne fait plus
//! qu'assembler ce store avec le registre des méthodes de vote
//! ([`VotingMethodRegistry`]), et exposer les deux ensemble aux
//! handlers HTTP (`routes.rs`) sous une seule structure `AppState`.
//!
//! ## Historique
//!
//! Ce fichier contenait auparavant directement un
//! `Mutex<HashMap<...>>` (cf. `docs/DEVLOG.md`, première itération).
//! Ce `Mutex` a été déplacé tel quel dans
//! `agoravote-store::memory::MemoryStore` au moment d'introduire le
//! backend PostgreSQL (`agoravote-store::postgres::PgStore`), pour
//! que les deux backends soient interchangeables derrière le même
//! type `agoravote_store::Store` — cf. `docs/ARCHITECTURE.md`.

use agoravote_store::Store;
use agoravote_voting::VotingMethodRegistry;

/// État partagé entre toutes les requêtes HTTP : persistance +
/// catalogue des méthodes de vote disponibles.
#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    /// Le registre de méthodes de vote est immuable après démarrage :
    /// pas besoin qu'il transite par le store.
    pub voting_methods: VotingMethodRegistry,
}

impl AppState {
    /// Backend en mémoire — cf. `docs/ARCHITECTURE.md` (tests,
    /// démonstration locale sans dépendance externe).
    pub fn new_memory() -> Self {
        Self {
            store: Store::new_memory(),
            voting_methods: VotingMethodRegistry::with_builtin_methods(),
        }
    }

    /// Backend PostgreSQL — cf. `agoravote_store::Store::connect_postgres`.
    pub async fn new_postgres(database_url: &str) -> Result<Self, agoravote_store::StoreError> {
        Ok(Self {
            store: Store::connect_postgres(database_url).await?,
            voting_methods: VotingMethodRegistry::with_builtin_methods(),
        })
    }
}
