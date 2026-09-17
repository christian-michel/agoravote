import type { TranslationKey } from "../i18n/translations/fr";

/**
 * Les 6 mentions du jugement majoritaire (valeurs 1 à 6, cf.
 * `agoravote-voting::majority_judgment` côté serveur, qui ne connaît
 * que ces entiers — libellés et couleurs sont un choix d'interface,
 * pas une donnée renvoyée par l'API, même principe que
 * `methodCopy.ts`). Couleurs reprises telles quelles d'une référence
 * externe (jugement majoritaire à 6 mentions) pour que la palette
 * rouge → vert soit immédiatement reconnaissable comme une échelle de
 * sentiment, pas une série catégorielle arbitraire — fixes, donc
 * indépendantes de la langue d'interface, contrairement aux libellés
 * (résolus via `t()`, cf. `mentionLabel` ci-dessous).
 */
export interface MajorityJudgmentMentionColor {
  value: number;
  labelKey: TranslationKey;
  color: string;
}

export const MAJORITY_JUDGMENT_MENTIONS: MajorityJudgmentMentionColor[] = [
  { value: 1, labelKey: "mj.mention1", color: "#d32f2f" },
  { value: 2, labelKey: "mj.mention2", color: "#ff9800" },
  { value: 3, labelKey: "mj.mention3", color: "#ffc107" },
  { value: 4, labelKey: "mj.mention4", color: "#8bc34a" },
  { value: 5, labelKey: "mj.mention5", color: "#4caf50" },
  { value: 6, labelKey: "mj.mention6", color: "#9e9e9e" },
];

export function mentionLabelKey(value: number): TranslationKey {
  return MAJORITY_JUDGMENT_MENTIONS.find((m) => m.value === value)?.labelKey ?? "mj.mention3";
}

export function mentionColor(value: number): string {
  return MAJORITY_JUDGMENT_MENTIONS.find((m) => m.value === value)?.color ?? "#9e9e9e";
}
