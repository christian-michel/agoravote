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
/// citoyen", §10.1). On accepte soit une liste de sélections (choix
/// unique/multiple/classement), soit une table de scores (vote par
/// score) — jamais les deux à la fois, cf. validation dans le handler.
#[derive(Debug, Deserialize, Default)]
pub struct CastBallotRequest {
    #[serde(default)]
    pub voter_id: Option<Id>,
    #[serde(default)]
    pub selections: Vec<String>,
    #[serde(default)]
    pub scores: Option<HashMap<String, f64>>,
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
