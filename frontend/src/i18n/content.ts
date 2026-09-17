import type { LanguageCode } from "./LanguageContext";

/**
 * Résout un contenu multilingue STOCKÉ (`prompt` de question, `labels`
 * d'option — `Record<code langue, texte>`, cf. cahier des charges
 * §1.1 "multilinguisme natif" et `agoravote_core::form::QuestionOption`)
 * selon la langue d'interface courante.
 *
 * Ordre de repli : langue courante -> français (langue toujours
 * requise à la saisie côté `FormBuilder`, cf. sa doc) -> première
 * traduction disponible -> chaîne vide. Ne lève jamais d'exception :
 * un contenu partiellement traduit doit rester affichable en dégradé,
 * pas casser l'écran — distinct des erreurs serveur qui, elles,
 * doivent être loguées (§13) : ceci est un affichage client résilient
 * face à des données incomplètes, pas une erreur à signaler.
 */
export function resolveLocalizedText(
  record: Record<string, string> | undefined | null,
  lang: LanguageCode,
): string {
  if (!record) return "";
  if (record[lang]) return record[lang];
  if (record.fr) return record.fr;
  for (const value of Object.values(record)) {
    if (value) return value;
  }
  return "";
}
