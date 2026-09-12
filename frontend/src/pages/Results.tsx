import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { api, ApiError, type ResultSet } from "../api/client";
import { Alert, Card } from "../components/ui";

export default function Results() {
  const { campaignId, questionId } = useParams<{ campaignId: string; questionId: string }>();
  const [result, setResult] = useState<ResultSet | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

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

  return (
    <div className="mx-auto max-w-lg">
      <p className="text-xs font-medium uppercase tracking-wide text-muted">Résultat</p>
      <h1 className="mt-2 font-display text-2xl font-medium text-ink">
        {result.outcome.winners.length > 0 ? result.outcome.winners.join(", ") : "Aucun gagnant"}
      </h1>

      <Card className="mt-6">
        <ul className="flex flex-col gap-3">
          {sorted.map(([option, pct]) => (
            <li key={option}>
              <div className="flex items-center justify-between text-sm">
                <span
                  className={
                    result.outcome.winners.includes(option) ? "font-medium text-ink" : "text-ink-soft"
                  }
                >
                  {option}
                </span>
                <span className="text-muted">{pct.toFixed(1)}%</span>
              </div>
              <div className="mt-1 h-2 w-full overflow-hidden rounded-full bg-line/60">
                <div
                  className="h-full bg-accent"
                  style={{ width: `${Math.min(100, Math.max(0, pct))}%` }}
                />
              </div>
            </li>
          ))}
        </ul>

        <div className="mt-5 flex flex-wrap gap-x-6 gap-y-1 border-t border-line pt-4 text-xs text-muted">
          <span>
            {result.outcome.valid_ballots} suffrage(s) exprimé(s) sur {result.outcome.total_ballots}{" "}
            bulletin(s)
          </span>
          <span>
            Méthode {result.voting_method_id} v{result.voting_method_version}
          </span>
          {result.outcome.quorum_met !== null && result.outcome.quorum_met !== undefined && (
            <span>Quorum {result.outcome.quorum_met ? "atteint" : "non atteint"}</span>
          )}
        </div>
      </Card>
    </div>
  );
}
