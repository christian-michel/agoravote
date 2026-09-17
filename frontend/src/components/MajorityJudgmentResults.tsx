import type { ResultSet } from "../api/client";
import { MAJORITY_JUDGMENT_MENTIONS, mentionColor, mentionLabel } from "../lib/majorityJudgmentMentions";

/** Reflète `MentionBreakdown` côté Rust
 * (`agoravote_voting::majority_judgment`) — cf. sa doc pour le détail
 * du calcul. Casté depuis `TallyOutcome.metadata["mention_breakdown"]`,
 * qui reste typé `unknown` côté schéma OpenAPI (`additionalProperties`
 * libre, cf. doc de `TallyOutcome::metadata`) : cette forme est un
 * contrat entre CE composant et le module serveur, pas une garantie
 * d'API générale. */
interface MentionBreakdown {
  counts: Record<string, number>;
  percentages: Record<string, number>;
  median_mention: number;
  percentage_at_or_above_median: number;
  percentage_below_median: number;
}

/** Affichage des résultats "jugement majoritaire" (§6.2, §15.1) :
 * barre empilée par mention (couleurs dédiées, cf.
 * `majorityJudgmentMentions.ts`), ligne de médiane, mention majoritaire
 * et badge "(≥X %)", classement par médiane décroissante puis
 * pourcentage de départage — reproduit la lecture proposée par la
 * référence externe qui a inspiré cette méthode (cf. docs/DEVLOG.md).
 */
export function MajorityJudgmentResults({
  result,
  optionLabel,
}: {
  result: ResultSet;
  optionLabel: (optionId: string) => string;
}) {
  const breakdowns = (result.outcome.metadata["mention_breakdown"] ?? {}) as Record<
    string,
    MentionBreakdown
  >;

  const options = Object.keys(result.outcome.counts).sort((a, b) => {
    const medianDiff = result.outcome.counts[b] - result.outcome.counts[a];
    if (medianDiff !== 0) return medianDiff;
    return result.outcome.percentages[b] - result.outcome.percentages[a];
  });

  return (
    <div className="flex flex-col gap-4">
      {options.map((optionId, index) => {
        const breakdown = breakdowns[optionId];
        const median = result.outcome.counts[optionId];
        const badge = Math.round(result.outcome.percentages[optionId]);
        const totalVotes = breakdown
          ? Object.values(breakdown.counts).reduce((sum, n) => sum + n, 0)
          : 0;

        return (
          <div key={optionId} className="rounded-lg border border-line bg-paper p-4">
            <div className="flex flex-wrap items-center gap-3">
              <span className="text-lg font-bold text-ink">#{index + 1}</span>
              <span className="flex-1 font-medium text-ink">{optionLabel(optionId)}</span>
              <span
                className="rounded-full px-3 py-1 text-sm font-semibold"
                style={{
                  backgroundColor: `color-mix(in srgb, ${mentionColor(median)} 18%, white)`,
                  color: mentionColor(median),
                }}
              >
                {mentionLabel(median)}
              </span>
              <span className="text-xs italic text-muted">(≥{badge}&nbsp;%)</span>
            </div>

            <div className="relative mt-4 h-9">
              <div className="flex h-full overflow-hidden rounded-md border border-line">
                {MAJORITY_JUDGMENT_MENTIONS.map((mention) => {
                  const pct = breakdown?.percentages[String(mention.value)] ?? 0;
                  if (pct <= 0) return null;
                  return (
                    <div
                      key={mention.value}
                      title={`${mention.label} : ${Math.round(pct)} %`}
                      className="flex items-center justify-center text-[11px] font-semibold text-white"
                      style={{ width: `${pct}%`, backgroundColor: mention.color }}
                    >
                      {pct >= 12 ? `${Math.round(pct)}%` : ""}
                    </div>
                  );
                })}
              </div>
              <div
                className="pointer-events-none absolute -top-2 bottom-0 left-1/2 w-0.5 -translate-x-1/2"
                style={{
                  backgroundImage:
                    "repeating-linear-gradient(to bottom, var(--color-ink-soft) 0, var(--color-ink-soft) 4px, transparent 4px, transparent 8px)",
                }}
              >
                <span className="absolute -top-2 left-1/2 -translate-x-1/2 whitespace-nowrap rounded border border-line bg-surface px-1.5 py-0.5 text-[10px] font-bold text-ink-soft">
                  ▼ MÉDIANE
                </span>
              </div>
            </div>

            <p className="mt-2 text-right text-xs italic text-muted">
              {totalVotes} vote{totalVotes > 1 ? "s" : ""} au total
            </p>
          </div>
        );
      })}

      <div className="rounded-md border border-line bg-surface p-3">
        <p className="text-xs font-medium uppercase tracking-wide text-muted">
          Légende des mentions
        </p>
        <div className="mt-2 flex flex-wrap gap-3">
          {MAJORITY_JUDGMENT_MENTIONS.map((mention) => (
            <span key={mention.value} className="flex items-center gap-1.5 text-xs text-ink-soft">
              <span
                className="h-3 w-5 rounded-sm border border-line"
                style={{ backgroundColor: mention.color }}
              />
              {mention.label}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}

