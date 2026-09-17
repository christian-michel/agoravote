import type { TranslationKey } from "../i18n/translations/fr";

/**
 * Libellés/descriptions éditoriales pour les méthodes du catalogue
 * natif — texte d'interface, pas une donnée renvoyée par l'API (le
 * manifeste ne porte qu'un id/version/kind/entrées/sorties, cf.
 * docs/openapi.yaml `ModuleManifest`). Partagé entre `ModulesPage`,
 * `TallyPanel`, `Results` et `Analysis` pour ne pas dupliquer ce texte
 * à plusieurs endroits. Clés de traduction plutôt que texte en dur :
 * résolues via `t()` au point d'appel (cf. `getMethodCopy`).
 */
const METHOD_COPY_KEYS: Record<string, { labelKey: TranslationKey; descriptionKey: TranslationKey }> = {
  "voting.majority": {
    labelKey: "method.majority.label",
    descriptionKey: "method.majority.description",
  },
  "voting.approval": {
    labelKey: "method.approval.label",
    descriptionKey: "method.approval.description",
  },
  "voting.score": {
    labelKey: "method.score.label",
    descriptionKey: "method.score.description",
  },
  "voting.majority_judgment": {
    labelKey: "method.majorityJudgment.label",
    descriptionKey: "method.majorityJudgment.description",
  },
};

export function getMethodCopy(
  methodId: string,
  t: (key: TranslationKey) => string,
): { label: string; description: string } | undefined {
  const keys = METHOD_COPY_KEYS[methodId];
  if (!keys) return undefined;
  return { label: t(keys.labelKey), description: t(keys.descriptionKey) };
}
