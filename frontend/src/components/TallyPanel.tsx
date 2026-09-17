import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, ApiError, type ModuleManifest, type Question, type ResultSet } from "../api/client";
import { useLanguage } from "../i18n/LanguageContext";
import { resolveLocalizedText } from "../i18n/content";
import { Alert, Button, Card } from "./ui";
import { BarList } from "./charts";
import { getMethodCopy } from "../lib/methodCopy";

export function TallyPanel({
  campaignId,
  question,
  modules,
  canTally,
}: {
  campaignId: string;
  question: Question;
  modules: ModuleManifest[];
  /** Le dépouillement n'a de sens qu'une fois la campagne clôturée ou
   * au moins publiée — cf. le garde-fou déjà appliqué côté API
   * (`tally` fonctionne dès qu'il y a des bulletins, mais dépouiller
   * une campagne encore ouverte donnerait un résultat partiel qui
   * pourrait induire en erreur s'il est confondu avec un résultat
   * final). */
  canTally: boolean;
}) {
  const { t, lang } = useLanguage();
  const [methodId, setMethodId] = useState(modules[0]?.id ?? "");
  const [eligibleVoters, setEligibleVoters] = useState("");
  const [quorum, setQuorum] = useState("");
  const [result, setResult] = useState<ResultSet | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    api
      .getResults(campaignId, question.id)
      .then(setResult)
      .catch(() => {
        // Pas encore de résultat calculé pour cette question — état
        // normal, pas une erreur à afficher (cf. ApiError, 404
        // silencieusement ignoré ici).
      });
  }, [campaignId, question.id]);

  async function handleTally() {
    setError(null);
    setLoading(true);
    try {
      const outcome = await api.tally(campaignId, question.id, {
        voting_method_id: methodId,
        eligible_voters: eligibleVoters ? Number(eligibleVoters) : undefined,
        quorum: quorum ? Number(quorum) / 100 : undefined,
        seats: 1,
      });
      setResult(outcome);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t("tally.genericError"));
    } finally {
      setLoading(false);
    }
  }

  const options =
    "options" in question.question_type
      ? (question.question_type.options as { id: string; labels: Record<string, string> }[])
      : [];
  // Les résultats calculés (`percentages`/`winners`) sont indexés par
  // ID d'option brut (ex. "bleu-ciel"), pas fait pour être lu tel quel
  // par une organisatrice — cf. Results.tsx, même correction.
  const optionLabel = (optionId: string) => {
    const option = options.find((o) => o.id === optionId);
    return option ? resolveLocalizedText(option.labels, lang) : optionId;
  };

  return (
    <Card>
      <div className="flex items-start justify-between gap-4">
        <h3 className="font-medium text-ink">{resolveLocalizedText(question.prompt, lang)}</h3>
        <Link
          to={`/campagnes/${campaignId}/questions/${question.id}/resultats`}
          className="shrink-0 text-xs text-accent hover:underline"
        >
          {t("tally.publicResultsLink")}
        </Link>
      </div>

      {error && (
        <div className="mt-3">
          <Alert>{error}</Alert>
        </div>
      )}

      {canTally && (
        <div className="mt-5 border-t border-line pt-4">
          <p className="text-xs font-medium uppercase tracking-wide text-muted">{t("tally.method")}</p>
          <div className="mt-2 grid gap-2 sm:grid-cols-3">
            {modules.map((m) => {
              const copy = getMethodCopy(m.id, t);
              const selected = methodId === m.id;
              return (
                <button
                  key={m.id}
                  type="button"
                  onClick={() => setMethodId(m.id)}
                  className={`rounded-lg border px-3 py-2.5 text-left transition-colors ${
                    selected ? "border-accent bg-accent-soft" : "border-line hover:border-accent/50"
                  }`}
                >
                  <p className="text-sm font-medium text-ink">{copy?.label ?? m.id}</p>
                  {copy && <p className="mt-0.5 text-xs text-muted">{copy.description}</p>}
                </button>
              );
            })}
          </div>

          {options.length > 0 && (
            <div className="mt-4">
              <p className="text-xs font-medium uppercase tracking-wide text-muted">
                {t("tally.ballotPreview")}
              </p>
              <ul className="mt-2 flex flex-col gap-1.5">
                {options.map((o) => (
                  <li
                    key={o.id}
                    className="rounded-md border border-line bg-paper px-3 py-1.5 text-xs text-ink-soft"
                  >
                    {resolveLocalizedText(o.labels, lang)}
                  </li>
                ))}
              </ul>
            </div>
          )}

          <div className="mt-4 flex flex-wrap items-end gap-3 text-sm">
            <label className="flex flex-col gap-1">
              <span className="text-xs font-medium text-ink-soft">{t("tally.eligibleVoters")}</span>
              <input
                className="w-28 rounded-md border border-line bg-surface px-2 py-1.5"
                type="number"
                min={0}
                value={eligibleVoters}
                onChange={(e) => setEligibleVoters(e.target.value)}
              />
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-medium text-ink-soft">{t("tally.quorumPercent")}</span>
              <input
                className="w-24 rounded-md border border-line bg-surface px-2 py-1.5"
                type="number"
                min={0}
                max={100}
                value={quorum}
                onChange={(e) => setQuorum(e.target.value)}
              />
            </label>
            <Button onClick={handleTally} disabled={loading || !methodId}>
              {loading ? t("tally.computing") : t("tally.launch")}
            </Button>
          </div>
        </div>
      )}

      {result && (
        <div className="mt-5 border-t border-line pt-4">
          <p className="text-xs text-muted">
            {t("results.methodVersion", {
              label: getMethodCopy(result.voting_method_id, t)?.label ?? result.voting_method_id,
              version: result.voting_method_version,
            })}{" "}
            · {t("tally.validBallots", {
              valid: result.outcome.valid_ballots,
              total: result.outcome.total_ballots,
            })}
          </p>
          <div className="mt-3">
            <BarList
              items={Object.entries(result.outcome.percentages)
                .sort(([, a], [, b]) => b - a)
                .map(([option, pct]) => ({
                  key: option,
                  label: optionLabel(option),
                  value: pct,
                  highlighted: result.outcome.winners.includes(option),
                }))}
            />
          </div>
          {result.outcome.quorum_met !== null && result.outcome.quorum_met !== undefined && (
            <p className="mt-3 text-xs text-muted">
              {result.outcome.quorum_met ? t("results.quorumMet") : t("results.quorumNotMet")}
            </p>
          )}
        </div>
      )}
    </Card>
  );
}
