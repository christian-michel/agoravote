//! Structures de requête/réponse de l'API HTTP.
//!
//! On distingue volontairement ces DTOs (Data Transfer Objects) des
//! entités du domaine (`agoravote_core::Campaign`, etc.) : le format
//! JSON exposé à l'extérieur (§14 "Interopérabilité et standards")
//! n'a pas à évoluer au même rythme ni de la même façon que le modèle
//! interne. Pour ce prototype, les deux se ressemblent beaucoup, mais
//! la frontière existe déjà pour que le modèle interne reste libre de
//! changer sans casser le contrat d'API.

use std::collections::HashMap;

use agoravote_core::{Id, QuestionType};
use serde::{Deserialize, Serialize};

/// Corps de requête : création d'une campagne (écran "01/02", §10.1).
#[derive(Debug, Deserialize)]
pub struct CreateCampaignRequest {
    pub organization_id: Id,
    pub title: String,
}

/// Corps de requête : création du formulaire d'une campagne
/// (écran "03. Éditeur de formulaire", §10.1). On simplifie ici en
/// acceptant directement la liste des questions à créer, plutôt que
/// de reproduire toute la logique conditionnelle (§10.1, écran 05,
/// hors périmètre de ce prototype).
#[derive(Debug, Deserialize)]
pub struct CreateFormRequest {
    pub questions: Vec<CreateQuestionRequest>,
}

#[derive(Debug, Deserialize)]
pub struct CreateQuestionRequest {
    pub prompt: String,
    #[serde(flatten)]
    pub question_type: QuestionType,
    #[serde(default)]
    pub required: bool,
}

/// Corps de requête : soumission d'un bulletin (écran "09. Vote
/// citoyen", §10.1). Lequel des quatre champs de données est
/// pertinent dépend du [`QuestionType`] de la question répondue — le
/// handler (`routes.rs::cast_ballot`) relit ce type pour choisir le
/// bon champ et valider son contenu (bornes, longueur, classement
/// complet), plutôt que de faire confiance au client sur quel champ a
/// été rempli.
#[derive(Debug, Deserialize, Default)]
pub struct CastBallotRequest {
    #[serde(default)]
    pub voter_id: Option<Id>,
    /// `SingleChoice`/`MultipleChoice` (id de la ou des options
    /// choisies) ou `Ranking` (ids de toutes les options, dans
    /// l'ordre de préférence).
    #[serde(default)]
    pub selections: Vec<String>,
    /// Réservé pour `voting.score` (§8.2, déjà implémenté dans
    /// `agoravote-voting` et testé, cf. son crate), jamais encore
    /// câblé à un `QuestionType` : aucune question ne produit
    /// aujourd'hui de bulletin avec ce champ rempli (cf.
    /// `routes.rs::build_ballot`) — même statut que
    /// `agoravote_store::Store::get_form`, gardé prêt plutôt que
    /// supprimé. `#[allow(dead_code)]` : jamais lu par du code Rust
    /// aujourd'hui, mais bien désérialisé (`Deserialize`) si un client
    /// le fournit malgré tout — pas du code mort au sens propre.
    #[serde(default)]
    #[allow(dead_code)]
    pub scores: Option<HashMap<String, f64>>,
    /// `Number`/`Scale`.
    #[serde(default)]
    pub numeric_value: Option<f64>,
    /// `Text`.
    #[serde(default)]
    pub text_value: Option<String>,
}

/// Réponse à `GET /campaigns/:id/questions/:qid/responses` — pour les
/// types de question qu'aucune [`agoravote_voting::VotingMethod`] ne
/// sait dépouiller (`Text`, `Number`/`Scale`, `Ranking`) : il n'y a
/// pas de "gagnant" à calculer, seulement des réponses brutes à
/// consulter telles quelles (§5.1) — pour `Number`/`Scale`, accompagnées
/// d'un résumé statistique **descriptif** (`agoravote_stats`, cf. §6.2)
/// explicitement distinct d'un résultat de vote (même principe que la
/// séparation de crate `agoravote-stats`/`agoravote-voting`, cf. doc
/// de `agoravote_stats`). Jamais utilisée pour `SingleChoice`/
/// `MultipleChoice` (qui passent par `/tally` puis `/results`) — le
/// handler renvoie une erreur explicite dans ce cas plutôt que de
/// laisser deviner quelle route utiliser.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QuestionResponses {
    Text {
        values: Vec<String>,
    },
    Numeric {
        values: Vec<f64>,
        summary: Option<NumericSummary>,
    },
    Ranking {
        rankings: Vec<Vec<String>>,
    },
}

/// Résumé statistique descriptif d'un ensemble de valeurs numériques
/// (`Number`/`Scale`) — cf. `agoravote_stats::descriptive`. `None`
/// pour chaque champ dès que l'échantillon est vide (cf. doc de ces
/// fonctions : une moyenne/médiane/écart-type n'a pas de sens sur un
/// ensemble vide, jamais renvoyée comme `0.0` qui serait trompeur).
#[derive(Debug, Serialize)]
pub struct NumericSummary {
    pub count: usize,
    pub mean: Option<f64>,
    pub median: Option<f64>,
    pub std_dev: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// Corps de requête : lancement d'un dépouillement (écran "06/11",
/// §10.1). `voting_method_id` doit correspondre à un module enregistré
/// dans le [`agoravote_voting::VotingMethodRegistry`] (ex:
/// "voting.majority").
#[derive(Debug, Deserialize)]
pub struct TallyRequest {
    pub voting_method_id: String,
    #[serde(default)]
    pub eligible_voters: Option<u64>,
    #[serde(default)]
    pub quorum: Option<f64>,
    #[serde(default = "default_seats")]
    pub seats: u32,
}

fn default_seats() -> u32 {
    1
}

/// Corps de requête : inscription (§13). Crée à la fois un `User`
/// (profil) et un `Account` (identifiants) — cf. la doc de
/// `agoravote_core::auth` pour la raison de cette séparation.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub organization_id: Id,
    pub email: String,
    pub password: String,
    pub display_name: String,
}

/// Corps de requête : connexion (§13).
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Réponse à une inscription ou une connexion réussie. `token` n'est
/// renvoyé qu'ICI, une seule fois — cf. la doc de
/// `agoravote_core::auth::Session` sur le fait qu'il n'est jamais
/// stocké en clair côté serveur après cet instant. Le client est
/// responsable de le conserver (ex : pour l'inclure dans l'en-tête
/// `Authorization: Bearer <token>` des requêtes suivantes).
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: agoravote_core::User,
}

/// Réponse à `POST /auth/g1/challenge` — cf. `docs/G1_INTEGRATION.md`
/// §4. Reprend directement les deux champs d'`agoravote_g1::Challenge`
/// (pas de `#[serde(flatten)]` sur le type importé : ce DTO reste
/// libre d'évoluer indépendamment du type interne du crate `agoravote-g1`,
/// même principe que le reste de ce fichier, cf. doc de module).
#[derive(Debug, Serialize)]
pub struct G1ChallengeResponse {
    pub message: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl From<agoravote_g1::Challenge> for G1ChallengeResponse {
    fn from(challenge: agoravote_g1::Challenge) -> Self {
        Self {
            message: challenge.message,
            expires_at: challenge.expires_at,
        }
    }
}

/// Corps de requête : `POST /auth/g1/verify` (cf.
/// `docs/G1_INTEGRATION.md` §4). `message` doit être exactement le
/// texte renvoyé par `POST /auth/g1/challenge` — c'est lui qui porte
/// l'horodatage d'expiration vérifié par
/// `agoravote_g1::Challenge::verify_freshness` (cf. `routes.rs::g1_verify`).
///
/// `organization_id` n'est utilisé que lors de la toute première
/// vérification d'une clé publique Ğ1 encore inconnue : elle détermine
/// l'organisation du nouveau `User` auto-provisionné. Pour une clé déjà
/// liée à un compte existant, ce champ est ignoré (le `User` existant
/// et son organisation d'origine priment) — cf. le handler.
#[derive(Debug, Deserialize)]
pub struct G1VerifyRequest {
    pub organization_id: Id,
    pub public_key_hex: String,
    pub signature_hex: String,
    pub message: String,
}

/// Réponse d'erreur uniforme, pour que le client de l'API n'ait qu'un
/// seul format à gérer quel que soit le code HTTP renvoyé.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
        }
    }
}
