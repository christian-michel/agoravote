# syntax=docker/dockerfile:1
#
# Build multi-étapes : une image "builder" lourde (toolchain Rust
# complète) compile le binaire, puis une image finale minimale ne
# contient QUE le binaire compilé et ses dépendances système
# strictement nécessaires à l'exécution. Résultat : l'image livrée ne
# contient ni compilateur, ni sources, ni Cargo — une surface
# d'attaque bien plus réduite qu'une image "tout-en-un" (cf.
# docs/SECURITY.md).

# ---------------------------------------------------------------------
# Étape 1 : builder
# ---------------------------------------------------------------------
FROM rust:1-slim-bookworm AS builder

# Nécessaire pour compiler certaines dépendances transitives : `pkg-config`
# pour la détection de bibliothèques C en général, `libssl-dev` pour
# `native-tls` (utilisé par `agoravote-store`/`sqlx` pour la connexion
# PostgreSQL — cf. commentaire détaillé dans
# `crates/agoravote-store/Cargo.toml` sur le choix de ce backend TLS).
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# On copie d'abord uniquement les manifestes (Cargo.toml/Cargo.lock)
# pour profiter du cache de layers Docker : tant que les dépendances
# ne changent pas, `cargo fetch` n'est pas rejoué à chaque modification
# du code source, ce qui accélère nettement les reconstructions
# locales pendant le développement.
COPY Cargo.toml Cargo.lock ./
COPY crates/agoravote-core/Cargo.toml crates/agoravote-core/Cargo.toml
COPY crates/agoravote-voting/Cargo.toml crates/agoravote-voting/Cargo.toml
COPY crates/agoravote-stats/Cargo.toml crates/agoravote-stats/Cargo.toml
COPY crates/agoravote-store/Cargo.toml crates/agoravote-store/Cargo.toml
COPY crates/agoravote-api/Cargo.toml crates/agoravote-api/Cargo.toml

# Crée des fichiers source vides le temps de précompiler les
# dépendances (astuce standard pour maximiser le cache Docker sur un
# workspace Cargo à plusieurs crates). `agoravote-store` a en plus
# besoin de son dossier `migrations/` : la macro `sqlx::migrate!()`
# lit ces fichiers au moment de la COMPILATION (simple lecture locale,
# cf. commentaire dans `postgres.rs` — aucune base de données requise
# pendant `cargo build`), donc ce dossier doit exister dès cette étape,
# même vide, pour que la précompilation des dépendances réussisse.
RUN mkdir -p crates/agoravote-core/src crates/agoravote-voting/src \
             crates/agoravote-stats/src crates/agoravote-store/src \
             crates/agoravote-store/migrations crates/agoravote-api/src \
    && echo "fn main() {}" > crates/agoravote-api/src/main.rs \
    && echo "" > crates/agoravote-core/src/lib.rs \
    && echo "" > crates/agoravote-voting/src/lib.rs \
    && echo "" > crates/agoravote-stats/src/lib.rs \
    && echo "" > crates/agoravote-store/src/lib.rs \
    && cargo build --release -p agoravote-api \
    && rm -rf crates/*/src

# Copie le vrai code source (et les migrations SQL) et reconstruit —
# seule cette étape est invalidée quand le code applicatif change, pas
# la compilation des dépendances tierces faite juste au-dessus.
COPY crates crates
RUN touch crates/agoravote-core/src/lib.rs \
          crates/agoravote-voting/src/lib.rs \
          crates/agoravote-stats/src/lib.rs \
          crates/agoravote-store/src/lib.rs \
          crates/agoravote-api/src/main.rs \
    && cargo build --release -p agoravote-api

# ---------------------------------------------------------------------
# Étape 2 : image finale
# ---------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# `curl` : uniquement pour le HEALTHCHECK ci-dessous (interroge
# /health). `ca-certificates` : vérification des certificats TLS si un
# `DATABASE_URL` distant l'exige un jour. `libssl3` : bibliothèque
# partagée requise à l'exécution par `native-tls` (le binaire est
# dynamiquement lié à OpenSSL, cf. étape builder ci-dessus).
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*


# Utilisateur non privilégié : le processus ne tourne jamais en root
# dans le conteneur, même si le conteneur lui-même est compromis via
# une vulnérabilité du binaire — limite l'impact d'une éventuelle
# évasion de conteneur (cf. docs/SECURITY.md).
RUN groupadd --system agoravote && useradd --system --gid agoravote --no-create-home agoravote

COPY --from=builder /build/target/release/agoravote-api /usr/local/bin/agoravote-api

USER agoravote

# Niveau de log par défaut en conteneur : cf. docs/LOGGING.md pour le
# détail des niveaux disponibles. Surchargeable via `environment:` dans
# docker-compose.yml ou `docker run -e RUST_LOG=...`.
ENV RUST_LOG=info,agoravote_api=info,agoravote_core=info,agoravote_voting=info,agoravote_stats=info,agoravote_store=info

EXPOSE 3000

# Vérifie toutes les 10s que le serveur répond réellement sur /health
# (pas seulement que le process tourne) — Docker peut alors marquer le
# conteneur "unhealthy" et un orchestrateur redémarrer automatiquement
# un conteneur bloqué, plutôt que de le laisser servir des erreurs en
# silence.
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD curl --fail http://localhost:3000/health || exit 1

ENTRYPOINT ["/usr/local/bin/agoravote-api"]
