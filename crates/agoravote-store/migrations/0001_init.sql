-- Migration initiale — cf. cahier des charges §5 "Modèle de données
-- conceptuel" et §12.2 "Déploiement" (PostgreSQL en mode serveur).
--
-- Choix de schéma assumé : chaque entité est stockée comme une ligne
-- (id, données JSONB, métadonnées d'indexation minimales), plutôt
-- qu'un schéma relationnel entièrement normalisé (colonnes séparées
-- pour chaque champ). Raison : les types du domaine
-- (`agoravote_core::Campaign`, `Form`, `Ballot`, `ResultSet`) sont
-- déjà entièrement sérialisables en JSON (`serde`), et le sont DÉJÀ
-- exposés tels quels par l'API HTTP (`agoravote-api`) — dupliquer
-- cette structure en colonnes SQL séparées aurait demandé une couche
-- de mapping supplémentaire sans bénéfice pour le périmètre actuel.
-- Les colonnes non-JSONB ci-dessous (campaign_id, question_id...)
-- sont extraites du JSON uniquement pour permettre l'indexation et le
-- filtrage efficace (cf. §21 "travaux restants" : un schéma
-- pleinement relationnel avec contraintes d'intégrité référentielle
-- reste une évolution possible si des requêtes plus complexes
-- l'exigent un jour — non nécessaire pour le MVP).

CREATE TABLE campaigns (
    id          UUID PRIMARY KEY,
    data        JSONB NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE forms (
    id           UUID PRIMARY KEY,
    campaign_id  UUID NOT NULL,
    data         JSONB NOT NULL
);
CREATE INDEX idx_forms_campaign_id ON forms (campaign_id);

CREATE TABLE ballots (
    id           UUID PRIMARY KEY,
    campaign_id  UUID NOT NULL,
    question_id  UUID NOT NULL,
    data         JSONB NOT NULL,
    cast_at      TIMESTAMPTZ NOT NULL
);
-- Index composite : c'est exactement le filtre utilisé par
-- `VotingMethod::tally` au moment du dépouillement (cf.
-- `agoravote-api/src/routes.rs::tally_question`) — un dépouillement
-- ne doit pas dégénérer en balayage complet de la table à mesure que
-- le nombre de bulletins grandit.
CREATE INDEX idx_ballots_campaign_question ON ballots (campaign_id, question_id);

CREATE TABLE result_sets (
    -- Clé primaire = question_id : ce schéma ne conserve, comme le
    -- faisait le backend mémoire, que le DERNIER résultat calculé par
    -- question (cf. commentaire équivalent dans l'ancien
    -- `agoravote-api/src/state.rs`). Conserver l'historique complet
    -- des recalculs est une évolution possible (§5.1 : un `ResultSet`
    -- n'est jamais modifié, seulement remplacé) mais demanderait de
    -- lever cette contrainte d'unicité — non nécessaire aujourd'hui.
    question_id  UUID PRIMARY KEY,
    campaign_id  UUID NOT NULL,
    data         JSONB NOT NULL,
    computed_at  TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_result_sets_campaign_id ON result_sets (campaign_id);
