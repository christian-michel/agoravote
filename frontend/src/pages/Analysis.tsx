import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { api, ApiError, type Campaign, type Form, type Question, type ResultSet } from "../api/client";
import { useLanguage } from "../i18n/LanguageContext";
import { resolveLocalizedText } from "../i18n/content";
import { Alert, Card } from "../components/ui";
import { BarList } from "../components/charts";
import { getMethodCopy } from "../lib/methodCopy";

/**
 * Écran "Analyse et visualisation" (planche 6, §10.1 écran 12).
 *
 * Volontairement plus modeste que la planche fournie : celle-ci montre
 * une évolution temporelle de la participation et des comparaisons par
 * groupe d'âge/territoire — aucune de ces deux dimensions n'existe
 * dans le modèle de données actuel (les bulletins n'ont ni horodatage
 * exploité en série, ni attribut démographique, cf.
 * `agoravote_core::ballot::Ballot`), et `agoravote-stats` (moyenne,
 * médiane, écart-type...) n'est câblé à aucune route de l'API. Plutôt
 * que d'inventer ces graphiques avec des données fictives — contraire
 * à la règle du projet "jamais de donnée fictive présentée comme
 * réelle" — cet écran se limite à ce qui est réellement mesurable
 * aujourd'hui : la comparaison des résultats déjà calculés entre les
 * questions d'une même campagne. Le reste (démographie, séries
 * temporelles, statistiques descriptives) est noté comme limite
 * connue dans docs/ROADMAP.md plutôt que simulé.
 */
export default function Analysis() {
  const { t, lang } = useLanguage();
  const { campaignId } = useParams<{ campaignId: string }>();
  const [campaign, setCampaign] = useState<Campaign | null>(null);
  const [form, setForm] = useState<Form | null>(null);
  const [results, setResults] = useState<Record<string, ResultSet | null>>({});
  const [view, setView] = useState<"graphique" | "tableau">("graphique");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!campaignId) return;
    (async () => {
      setLoading(true);
      setError(null);
      try {
        const [c, f] = await Promise.all([api.getCampaign(campaignId), api.getForm(campaignId)]);
        setCampaign(c);
        setForm(f);
        const entries = await Promise.all(
          f.questions.map(async (q) => {
            const r = await api.getResults(campaignId, q.id).catch(() => null);
            return [q.id, r] as const;
          }),
        );
        setResults(Object.fromEntries(entries));
      } catch (err) {
        setError(err instanceof ApiError ? err.message : t("analysis.loadError"));
      } finally {
        setLoading(false);
      }
    })();
  }, [campaignId, t]);

  if (loading) return <p className="text-sm text-muted">{t("common.loading")}</p>;
  if (error || !campaign || !form || !campaignId) return <Alert>{error ?? t("analysis.notFound")}</Alert>;

  const withResults = form.questions.filter((q) => results[q.id]);
  const totalValidBallots = withResults.reduce((sum, q) => sum + (results[q.id]?.outcome.valid_ballots ?? 0), 0);

  return (
    <div className="flex flex-col gap-8">
      <Link to={`/admin/campagnes/${campaignId}`} className="text-xs text-muted hover:text-ink">
        {t("common.back")} {campaign.title}
      </Link>

      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-ink">{t("analysis.title")}</h1>
          <p className="mt-1 text-sm text-muted">
            {t("analysis.subtitle", { title: campaign.title })}
          </p>
        </div>
        <div className="flex rounded-md border border-line p-0.5 text-xs">
          {(["graphique", "tableau"] as const).map((v) => (
            <button
              key={v}
              type="button"
              onClick={() => setView(v)}
              className={`rounded px-3 py-1.5 capitalize transition-colors ${
                view === v ? "bg-accent text-white" : "text-ink-soft hover:text-ink"
              }`}
            >
              {v === "graphique" ? t("analysis.viewChart") : t("analysis.viewTable")}
            </button>
          ))}
        </div>
      </div>

      <div className="grid gap-4 sm:grid-cols-3">
        <Card>
          <p className="text-2xl font-semibold text-ink">{form.questions.length}</p>
          <p className="text-sm text-muted">{t("analysis.statQuestions")}</p>
        </Card>
        <Card>
          <p className="text-2xl font-semibold text-ink">{withResults.length}</p>
          <p className="text-sm text-muted">{t("analysis.statTallied")}</p>
        </Card>
        <Card>
          <p className="text-2xl font-semibold text-ink">{totalValidBallots}</p>
          <p className="text-sm text-muted">{t("analysis.statValidBallots")}</p>
        </Card>
      </div>

      {withResults.length === 0 ? (
        <Card>
          <p className="text-sm text-muted">{t("analysis.none")}</p>
        </Card>
      ) : (
        <div className="grid gap-4 lg:grid-cols-2">
          {withResults.map((question: Question) => {
            const result = results[question.id];
            if (!result) return null;
            const sorted = Object.entries(result.outcome.percentages).sort(([, a], [, b]) => b - a);
            const method = getMethodCopy(result.voting_method_id, t);
            return (
              <Card key={question.id}>
                <h2 className="font-medium text-ink">{resolveLocalizedText(question.prompt, lang)}</h2>
                <p className="mt-0.5 text-xs text-muted">
                  {method?.label ?? result.voting_method_id} · {result.outcome.valid_ballots} suffrage(s)
                </p>
                {view === "graphique" ? (
                  <div className="mt-4">
                    <BarList
                      items={sorted.map(([option, pct]) => ({
                        key: option,
                        label: option,
                        value: pct,
                        highlighted: result.outcome.winners.includes(option),
                      }))}
                    />
                  </div>
                ) : (
                  <table className="mt-4 w-full text-sm">
                    <thead>
                      <tr className="border-b border-line text-left text-xs uppercase tracking-wide text-muted">
                        <th className="pb-2 font-medium">{t("analysis.colOption")}</th>
                        <th className="pb-2 text-right font-medium">{t("analysis.colBallots")}</th>
                        <th className="pb-2 text-right font-medium">{t("analysis.colPercent")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {sorted.map(([option, pct]) => (
                        <tr key={option} className="border-b border-line/60 last:border-0">
                          <td className="py-1.5 text-ink-soft">{option}</td>
                          <td className="py-1.5 text-right tabular-nums text-ink-soft">
                            {result.outcome.counts[option] ?? "—"}
                          </td>
                          <td className="py-1.5 text-right tabular-nums text-ink-soft">{pct.toFixed(1)}%</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}
