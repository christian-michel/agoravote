import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, ApiError, type ModuleManifest, type Question, type ResultSet } from "../api/client";
import { Alert, Button, Card } from "./ui";

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
      setError(err instanceof ApiError ? err.message : "Le dépouillement a échoué.");
    } finally {
      setLoading(false);
    }
  }

  return (
    <Card>
      <div className="flex items-start justify-between gap-4">
        <h3 className="font-display text-base font-medium text-ink">{question.prompt.fr}</h3>
        <Link
          to={`/campagnes/${campaignId}/questions/${question.id}/resultats`}
          className="shrink-0 text-xs text-accent hover:underline"
        >
          Page de résultats publique →
        </Link>
      </div>

      {error && (
        <div className="mt-3">
          <Alert>{error}</Alert>
        </div>
      )}

      {canTally && (
        <div className="mt-4 flex flex-wrap items-end gap-3 text-sm">
          <label className="flex flex-col gap-1">
            <span className="text-xs font-medium text-ink-soft">Méthode</span>
            <select
              className="rounded-md border border-line bg-surface px-2 py-1.5"
              value={methodId}
              onChange={(e) => setMethodId(e.target.value)}
            >
              {modules.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.id}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-xs font-medium text-ink-soft">Électeurs éligibles</span>
            <input
              className="w-28 rounded-md border border-line bg-surface px-2 py-1.5"
              type="number"
              min={0}
              value={eligibleVoters}
              onChange={(e) => setEligibleVoters(e.target.value)}
            />
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-xs font-medium text-ink-soft">Quorum (%)</span>
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
            {loading ? "Calcul…" : "Dépouiller"}
          </Button>
        </div>
      )}

      {result && (
        <div className="mt-5 border-t border-line pt-4">
          <p className="text-xs text-muted">
            Méthode {result.voting_method_id} v{result.voting_method_version} ·{" "}
            {result.outcome.valid_ballots} suffrage(s) exprimé(s) sur {result.outcome.total_ballots}
          </p>
          <ul className="mt-3 flex flex-col gap-2">
            {Object.entries(result.outcome.percentages)
              .sort(([, a], [, b]) => b - a)
              .map(([option, pct]) => (
                <li key={option}>
                  <div className="flex items-center justify-between text-sm">
                    <span className={result.outcome.winners.includes(option) ? "font-medium text-ink" : "text-ink-soft"}>
                      {option}
                      {result.outcome.winners.includes(option) && " — gagnant"}
                    </span>
                    <span className="text-muted">{pct.toFixed(1)}%</span>
                  </div>
                  <div className="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-line/60">
                    <div
                      className="h-full bg-accent"
                      style={{ width: `${Math.min(100, Math.max(0, pct))}%` }}
                    />
                  </div>
                </li>
              ))}
          </ul>
          {result.outcome.quorum_met !== null && result.outcome.quorum_met !== undefined && (
            <p className="mt-3 text-xs text-muted">
              Quorum {result.outcome.quorum_met ? "atteint" : "non atteint"}
            </p>
          )}
        </div>
      )}
    </Card>
  );
}
