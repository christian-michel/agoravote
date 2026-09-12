import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { api, ApiError, type ResultSet } from "../api/client";
import { Alert, Card } from "../components/ui";
import { BarList, Donut, ParticipationGauge } from "../components/charts";
import { InfoIcon, ShareIcon } from "../components/icons";
import { METHOD_COPY } from "../lib/methodCopy";

export default function Results() {
  const { campaignId, questionId } = useParams<{ campaignId: string; questionId: string }>();
  const [result, setResult] = useState<ResultSet | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [methodologyOpen, setMethodologyOpen] = useState(false);

  useEffect(() => {
    if (!campaignId || !questionId) return;
    setLoading(true);
    api
      .getResults(campaignId, questionId)
      .then(setResult)
      .catch((err) =>
        setError(
          err instanceof ApiError && err.status === 404
            ? "Aucun résultat n'a encore été calculé pour cette question."
            : "Impossible de charger les résultats.",
        ),
      )
      .finally(() => setLoading(false));
  }, [campaignId, questionId]);

  if (loading) return <p className="text-sm text-muted">Chargement…</p>;
  if (error || !result) {
    return (
      <div className="mx-auto max-w-md">
        <Alert kind="error">{error}</Alert>
      </div>
    );
  }

  const sorted = Object.entries(result.outcome.percentages).sort(([, a], [, b]) => b - a);
  const expressedRate =
    result.outcome.total_ballots > 0
      ? (result.outcome.valid_ballots / result.outcome.total_ballots) * 100
      : 0;
  const method = METHOD_COPY[result.voting_method_id];

  async function handleShare() {
    if (navigator.share) {
      await navigator.share({ url: window.location.href, title: "Résultats AgoraVote" }).catch(() => {});
    } else {
      await navigator.clipboard.writeText(window.location.href).catch(() => {});
    }
  }

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="text-xs font-medium uppercase tracking-wide text-muted">Résultats en direct</p>
          <h1 className="mt-1 text-2xl font-semibold text-ink">
            {result.outcome.winners.length > 0 ? result.outcome.winners.join(", ") : "Aucun gagnant"}
          </h1>
        </div>
        <button
          type="button"
          onClick={handleShare}
          className="flex shrink-0 items-center gap-2 rounded-md border border-line bg-surface px-3 py-2 text-xs text-ink-soft hover:border-accent"
        >
          <ShareIcon width={16} height={16} /> Partager
        </button>
      </div>

      <div className="grid gap-4 sm:grid-cols-2">
        <Card>
          <ParticipationGauge
            percent={expressedRate}
            label="Suffrages exprimés"
            sublabel={`${result.outcome.valid_ballots} sur ${result.outcome.total_ballots} bulletin(s) reçu(s)`}
          />
        </Card>
        <Card>
          <Donut
            slices={sorted.map(([option, pct]) => ({ key: option, label: option, value: pct }))}
          />
        </Card>
      </div>

      <Card>
        <h2 className="font-medium text-ink">Résultats par option</h2>
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

        <div className="mt-5 flex flex-wrap gap-x-6 gap-y-1 border-t border-line pt-4 text-xs text-muted">
          <span>Méthode {method?.label ?? result.voting_method_id} v{result.voting_method_version}</span>
          {result.outcome.quorum_met !== null && result.outcome.quorum_met !== undefined && (
            <span>Quorum {result.outcome.quorum_met ? "atteint" : "non atteint"}</span>
          )}
        </div>
      </Card>

      {method && (
        <Card>
          <button
            type="button"
            onClick={() => setMethodologyOpen((v) => !v)}
            className="flex w-full items-center gap-2 text-left"
          >
            <InfoIcon width={18} height={18} className="shrink-0 text-accent" />
            <span className="font-medium text-ink">Comprendre la méthode</span>
            <span className="ml-auto text-xs text-muted">{methodologyOpen ? "Réduire" : "Voir l'explication"}</span>
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
