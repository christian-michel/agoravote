import type { Question, QuestionResponses } from "../api/client";
import { useLanguage, pluralize } from "../i18n/LanguageContext";
import { resolveLocalizedText } from "../i18n/content";
import { BarList } from "./charts";

/**
 * Rendu des réponses brutes pour les types de question qu'aucune
 * méthode de vote ne dépouille (Texte, Nombre, Échelle, Classement) —
 * cf. `dto::QuestionResponses` côté API et sa doc sur la distinction
 * réponse brute / résultat calculé (§5.1). Partagé entre l'écran de
 * gestion (organisateur, `CampaignManage`) et la page de résultats
 * publique (`Results`) pour ne pas dupliquer cette logique de rendu.
 */
export function ResponsesView({
  responses,
  questionType,
}: {
  responses: QuestionResponses;
  questionType: Question["question_type"];
}) {
  const { t, lang } = useLanguage();

  if (responses.kind === "text") {
    if (responses.values.length === 0) {
      return <p className="text-sm text-muted">{t("responses.noneYet")}</p>;
    }
    return (
      <div className="flex flex-col gap-2">
        <p className="text-xs text-muted">
          {responses.values.length}{" "}
          {pluralize(responses.values.length, t("responses.responseSingular"), t("responses.responsePlural"))}
        </p>
        <ul className="flex flex-col gap-2">
          {responses.values.map((value, i) => (
            <li
              key={i}
              className="rounded-md border border-line bg-paper px-3 py-2 text-sm text-ink-soft"
            >
              {value || <span className="italic text-muted">{t("responses.emptyAnswer")}</span>}
            </li>
          ))}
        </ul>
      </div>
    );
  }

  if (responses.kind === "numeric") {
    const { summary } = responses;
    if (!summary) {
      return <p className="text-sm text-muted">{t("responses.noneYet")}</p>;
    }
    // Histogramme uniquement pour une échelle bornée (petit nombre de
    // valeurs entières possibles) : une question "Nombre" libre n'a
    // pas d'ensemble de valeurs fixe à regrouper de façon pertinente
    // sans une vraie logique de binning, hors périmètre ici — cf.
    // ROADMAP.
    const isScale = questionType.type === "scale";
    const histogram = isScale
      ? (() => {
          const min = questionType.min;
          const max = questionType.max;
          const counts = new Map<number, number>();
          for (const v of responses.values) counts.set(v, (counts.get(v) ?? 0) + 1);
          return Array.from({ length: max - min + 1 }, (_, i) => min + i).map((value) => ({
            key: String(value),
            label: String(value),
            value: counts.get(value) ?? 0,
          }));
        })()
      : null;

    return (
      <div className="flex flex-col gap-4">
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-5">
          <Stat label={t("responses.statCount")} value={summary.count} />
          <Stat label={t("responses.statMean")} value={summary.mean} />
          <Stat label={t("responses.statMedian")} value={summary.median} />
          <Stat label={t("responses.statMin")} value={summary.min} />
          <Stat label={t("responses.statMax")} value={summary.max} />
        </div>
        {histogram && <BarList items={histogram} unit={t("responses.unitAnswer")} />}
      </div>
    );
  }

  // responses.kind === "ranking"
  if (responses.rankings.length === 0) {
    return <p className="text-sm text-muted">{t("responses.noneRankingYet")}</p>;
  }
  const options =
    "options" in questionType
      ? new Map(questionType.options.map((o) => [o.id, resolveLocalizedText(o.labels, lang)]))
      : new Map<string, string>();
  return (
    <div className="flex flex-col gap-2">
      <p className="text-xs text-muted">
        {responses.rankings.length}{" "}
        {pluralize(responses.rankings.length, t("responses.rankingSingular"), t("responses.rankingPlural"))}{" "}
        — {t("responses.rankingRawNote")}
      </p>
      <ul className="flex flex-col gap-2">
        {responses.rankings.map((ranking, i) => (
          <li
            key={i}
            className="rounded-md border border-line bg-paper px-3 py-2 text-sm text-ink-soft"
          >
            {ranking.map((id, rank) => `${rank + 1}. ${options.get(id) ?? id}`).join(" · ")}
          </li>
        ))}
      </ul>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: number | null | undefined }) {
  return (
    <div className="rounded-md border border-line bg-paper px-3 py-2 text-center">
      <p className="text-lg font-semibold text-ink">
        {value != null ? Math.round(value * 100) / 100 : "—"}
      </p>
      <p className="text-xs text-muted">{label}</p>
    </div>
  );
}
