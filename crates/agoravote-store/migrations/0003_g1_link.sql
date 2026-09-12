-- Identité Ğ1v2 optionnelle (addendum v0.3 du cahier des charges) —
-- cf. `agoravote_core::auth::G1Link` et `docs/G1_INTEGRATION.md`.
--
-- Un utilisateur peut lier au plus un compte Ğ1 (contrainte UNIQUE sur
-- public_key_hex, pas sur user_id : un même user_id pourrait en
-- théorie lier plusieurs clés, mais rien dans ce prototype n'exige
-- l'inverse pour l'instant — cf. le commentaire de G1Link sur la
-- réutilisation de la logique d'unicité déjà appliquée à `accounts.email`).

CREATE TABLE g1_links (
    id               UUID PRIMARY KEY,
    user_id          UUID NOT NULL REFERENCES users (id),
    public_key_hex   TEXT NOT NULL UNIQUE,
    linked_at        TIMESTAMPTZ NOT NULL
);
