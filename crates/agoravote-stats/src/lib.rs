//! # agoravote-stats
//!
//! Moteur statistique **descriptif** d'AgoraVote — cf. cahier des
//! charges §6.2 (« Statistiques : comptage, fréquence, pourcentage,
//! moyenne, médiane, quartiles, variance, écart-type, croisements »)
//! et le glossaire §8.3/§8.4.
//!
//! Ce crate ne connaît ni les campagnes, ni les bulletins, ni les
//! méthodes de vote : il opère sur des vecteurs de valeurs numériques
//! ou catégorielles nues (`&[f64]`, `&[String]`). C'est volontaire —
//! cf. §8, dernier paragraphe : *« Le moteur statistique devra
//! distinguer les indicateurs descriptifs des interprétations »*. En
//! gardant ce crate agnostique du domaine électoral, on s'assure que
//! ses fonctions ne peuvent pas silencieusement "interpréter" un
//! résultat de vote — elles ne savent même pas qu'un vote existe. La
//! couche appelante (API, ou plus tard un crate `agoravote-analytics`)
//! reste seule responsable de choisir quelles données envoyer ici et
//! comment présenter le résultat.

pub mod crosstab;
pub mod descriptive;

pub use crosstab::cross_tabulation;
pub use descriptive::{quartiles, DescriptiveSummary};
