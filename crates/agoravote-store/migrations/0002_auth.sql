-- Migration §13 (authentification) — cf. cahier des charges §5, §13.
--
-- Même choix de schéma que la migration initiale (cf. son
-- commentaire) : stockage JSONB plutôt que colonnes entièrement
-- normalisées, sauf pour les colonnes qui doivent être indexées ou
-- contraintes (email unique, jointure session → utilisateur).

CREATE TABLE users (
    id    UUID PRIMARY KEY,
    data  JSONB NOT NULL
);

CREATE TABLE accounts (
    id             UUID PRIMARY KEY,
    user_id        UUID NOT NULL REFERENCES users (id),
    -- Unique : deux comptes ne peuvent pas partager un email (§13).
    -- Stocké déjà normalisé en minuscules par `agoravote-api`, cf.
    -- son commentaire — la contrainte ci-dessous est le filet de
    -- sécurité côté base, pas la seule protection.
    email          TEXT NOT NULL UNIQUE,
    password_hash  TEXT NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL
);

CREATE TABLE sessions (
    -- Clé primaire = empreinte du jeton (jamais le jeton en clair,
    -- cf. `agoravote_core::auth::Session` et `agoravote-auth`).
    token_hash       TEXT PRIMARY KEY,
    user_id          UUID NOT NULL REFERENCES users (id),
    organization_id  UUID NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL,
    expires_at       TIMESTAMPTZ NOT NULL
);
-- Accélère le nettoyage périodique des sessions expirées (cf. §21,
-- travaux restants : une tâche de purge n'est pas encore implémentée,
-- mais l'index est posé dès maintenant pour ne pas avoir à migrer la
-- table plus tard).
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);
