/**
 * Libellés/descriptions éditoriales pour les méthodes du catalogue
 * natif — texte d'interface, pas une donnée renvoyée par l'API (le
 * manifeste ne porte qu'un id/version/kind/entrées/sorties, cf.
 * docs/openapi.yaml `ModuleManifest`). Partagé entre `ModulesPage` et
 * `TallyPanel` pour ne pas dupliquer ce texte à deux endroits.
 */
export const METHOD_COPY: Record<string, { label: string; description: string }> = {
  "voting.majority": {
    label: "Majorité simple",
    description: "L'option qui obtient le plus de suffrages exprimés l'emporte.",
  },
  "voting.approval": {
    label: "Vote par approbation",
    description: "Chaque participant peut approuver plusieurs options sans les classer.",
  },
  "voting.score": {
    label: "Vote par score",
    description: "Chaque participant attribue une note à une ou plusieurs options ; la moyenne décide.",
  },
};
