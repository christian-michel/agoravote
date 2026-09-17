import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { api, ApiError, type Question, type QuestionResponses, type ResultSet } from "../api/client";
import { useLanguage } from "../i18n/LanguageContext";
import { resolveLocalizedText } from "../i18n/content";
import { Alert, Card } from "../components/ui";
import { BarList, Donut, ParticipationGauge } from "../components/charts";
import { ResponsesView } from "../components/ResponsesView";
import { MajorityJudgmentResults } from "../components/MajorityJudgmentResults";
import { InfoIcon, ShareIcon } from "../components/icons";
import { getMethodCopy } from "../lib/methodCopy";

export default function Results() {
  const { t, lang } = useLanguage();
  const { campaignId, questionId } = useParams<{ campaignId: string; questionId: string }>();
  const [question, setQuestion] = useState<Question | null>(null);
  const [result, setResult] = useState<ResultSet | null>(null);
  const [responses, setResponses] = useState<QuestionResponses | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [methodologyOpen, setMethodologyOpen] = useState(false);

  // Le TYPE de la question détermine quelle route interroger : les
  // questions à choix passent par /results (dépouillement par méthode
  // de vote) ; les autres par /responses (réponses brutes + résumé
  // descriptif, cf. `dto::QuestionResponses`) — récupérer la question
  // d'abord (via le formulaire) pour savoir laquelle appeler, et pour
  // résoudre les libellés d'options (l'identifiant brut d'une option,
  // ex. "bleu-ciel", n'est pas fait pour être lu tel quel par un
  // citoyen — cf. planche 5 du cahier des charges, qui montre des
  // libellés lisibles, pas des identifiants).
  useEffect(() => {
    if (!campaignId || !questionId) return;
    setLoading(true);
    setError(null);
    (async () => {
      try {
        const form = await api.getForm(campaignId);
        const q = form.questions.find((item) => item.id === questionId);
        if (!q) {
          setError(t("results.questionNotFound"));
          return;
        }
        setQuestion(q);

        const isComputed =
          q.question_type.type === "single_choice" ||
          q.question_type.type === "multiple_choice" ||
          q.question_type.type === "majority_judgment";
        if (isComputed) {
          try {
            setResult(await api.getResults(campaignId, questionId));
          } catch (err) {
            setError(
              err instanceof ApiError && err.status === 404
                ? t("results.notComputedYet")
                : t("results.loadError"),
            );
          }
        } else {
          setResponses(await api.getResponses(campaignId, questionId));
        }
      } catch (err) {
        setError(err instanceof ApiError ? err.message : t("results.loadQuestionError"));
      } finally {
        setLoading(false);
      }
    })();
  }, [campaignId, questionId, t]);

  async function handleShare() {
    if (navigator.share) {
      await navigator.share({ url: window.location.href, title: "AgoraVote" }).catch(() => {});
    } else {
      await navigator.clipboard.writeText(window.location.href).catch(() => {});
    }
  }

  if (loading) return <p className="text-sm text-muted">{t("common.loading")}</p>;
  if (error || !question) {
    return (
      <div className="mx-auto max-w-md">
        <Alert kind="error">{error}</Alert>
      </div>
    );
  }

  // --- Types sans méthode de dépouillement : réponses brutes ---
  if (!result) {
    return (
      <div className="mx-auto flex max-w-3xl flex-col gap-6">
        <div>
          <p className="text-xs font-medium uppercase tracking-wide text-muted">{t("results.liveTitle")}</p>
          <h1 className="mt-1 text-2xl font-semibold text-ink">
            {resolveLocalizedText(question.prompt, lang)}
          </h1>
        </div>
        <Card>
          {responses ? (
            <ResponsesView responses={responses} questionType={question.question_type} />
          ) : (
            <p className="text-sm text-muted">{t("results.noResponses")}</p>
          )}
        </Card>
      </div>
    );
  }

  // --- Choix unique/multiple : résultat calculé par méthode de vote ---
  const optionLabels: Record<string, string> =
    "options" in question.question_type
      ? Object.fromEntries(
          question.question_type.options.map((o) => [o.id, resolveLocalizedText(o.labels, lang)]),
        )
      : {};
  const label = (optionId: string) => optionLabels[optionId] ?? optionId;

  const sorted = Object.entries(result.outcome.percentages).sort(([, a], [, b]) => b - a);
  const expressedRate =
    result.outcome.total_ballots > 0
      ? (result.outcome.valid_ballots / result.outcome.total_ballots) * 100
      : 0;
  const method = getMethodCopy(result.voting_method_id, t);

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="text-xs font-medium uppercase tracking-wide text-muted">{t("results.liveTitle")}</p>
          <h1 className="mt-1 text-2xl font-semibold text-ink">
            {result.outcome.winners.length > 0
              ? result.outcome.winners.map(label).join(", ")
              : t("results.noWinner")}
          </h1>
        </div>
        <button
          type="button"
          onClick={handleShare}
          className="flex shrink-0 items-center gap-2 rounded-md border border-line bg-surface px-3 py-2 text-xs text-ink-soft hover:border-accent"
        >
          <ShareIcon width={16} height={16} /> {t("results.share")}
        </button>
      </div>

      <Card>
        <ParticipationGauge
          percent={expressedRate}
          label={t("results.expressedRate")}
          sublabel={t("results.ballotsReceived", {
            valid: result.outcome.valid_ballots,
            total: result.outcome.total_ballots,
          })}
        />
      </Card>

      {question.question_type.type === "majority_judgment" ? (
        <Card>
          <h2 className="font-medium text-ink">{t("results.byOption")}</h2>
          <div className="mt-4">
            <MajorityJudgmentResults result={result} optionLabel={label} />
          </div>
        </Card>
      ) : (
        <Card>
          <Donut
            slices={sorted.map(([option, pct]) => ({ key: option, label: label(option), value: pct }))}
          />
        </Card>
      )}

      {question.question_type.type !== "majority_judgment" && (
        <Card>
          <h2 className="font-medium text-ink">{t("results.byOption")}</h2>
          <div className="mt-4">
            <BarList
              items={sorted.map(([option, pct]) => ({
                key: option,
                label: label(option),
                value: pct,
                highlighted: result.outcome.winners.includes(option),
              }))}
            />
          </div>

          <div className="mt-5 flex flex-wrap gap-x-6 gap-y-1 border-t border-line pt-4 text-xs text-muted">
            <span>
              {t("results.methodVersion", {
                label: method?.label ?? result.voting_method_id,
                version: result.voting_method_version,
              })}
            </span>
            {result.outcome.quorum_met !== null && result.outcome.quorum_met !== undefined && (
              <span>{result.outcome.quorum_met ? t("results.quorumMet") : t("results.quorumNotMet")}</span>
            )}
          </div>
        </Card>
      )}

      {method && (
        <Card>
          <button
            type="button"
            onClick={() => setMethodologyOpen((v) => !v)}
            className="flex w-full items-center gap-2 text-left"
          >
            <InfoIcon width={18} height={18} className="shrink-0 text-accent" />
            <span className="font-medium text-ink">{t("results.understandMethod")}</span>
            <span className="ml-auto text-xs text-muted">
              {methodologyOpen ? t("results.collapse") : t("results.seeExplanation")}
            </span>
          </button>
          {methodologyOpen && (
            <p className="mt-3 text-sm text-ink-soft">
              <strong>{method.label}</strong> — {method.description}
            </p>
          )}
        </Card>
      )}
    </div>
  );
}
