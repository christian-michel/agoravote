/**
 * Les 6 mentions du jugement majoritaire (valeurs 1 à 6, cf.
 * `agoravote-voting::majority_judgment` côté serveur, qui ne connaît
 * que ces entiers — libellés et couleurs sont un choix d'interface,
 * pas une donnée renvoyée par l'API, même principe que
 * `methodCopy.ts`). Couleurs reprises telles quelles d'une référence
 * externe (jugement majoritaire à 6 mentions) pour que la palette
 * rouge → vert soit immédiatement reconnaissable comme une échelle de
 * sentiment, pas une série catégorielle arbitraire.
 */
export interface MajorityJudgmentMention {
  value: number;
  label: string;
  color: string;
}

export const MAJORITY_JUDGMENT_MENTIONS: MajorityJudgmentMention[] = [
  { value: 1, label: "Très défavorable", color: "#d32f2f" },
  { value: 2, label: "Défavorable", color: "#ff9800" },
  { value: 3, label: "Sans avis", color: "#ffc107" },
  { value: 4, label: "Favorable", color: "#8bc34a" },
  { value: 5, label: "Très favorable", color: "#4caf50" },
  { value: 6, label: "Ne sait pas", color: "#9e9e9e" },
];

export function mentionLabel(value: number): string {
  return MAJORITY_JUDGMENT_MENTIONS.find((m) => m.value === value)?.label ?? `Mention ${value}`;
}

export function mentionColor(value: number): string {
  return MAJORITY_JUDGMENT_MENTIONS.find((m) => m.value === value)?.color ?? "#9e9e9e";
}
