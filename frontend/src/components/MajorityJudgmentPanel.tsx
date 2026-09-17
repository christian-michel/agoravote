import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, ApiError, type Question, type ResultSet } from "../api/client";
import { Alert, Button, Card } from "./ui";
import { MajorityJudgmentResults } from "./MajorityJudgmentResults";

/**
 * Pendant de `TallyPanel` pour le jugement majoritaire : contrairement
 * aux questions à choix (plusieurs méthodes possibles, cf.
 * `TallyPanel`), une seule méthode s'applique ici
 * (`voting.majority_judgment` — c'est le format même du bulletin,
 * une mention par option, qui l'impose), donc pas de sélecteur de
 * méthode à afficher. Affichage dédié (barres par mention, pas le
 * `BarList` générique) — cf. `MajorityJudgmentResults`.
 */
export function MajorityJudgmentPanel({
  campaignId,
  question,
  canTally,
}: {
  campaignId: string;
  question: Question;
  canTally: boolean;
}) {
  const [result, setResult] = useState<ResultSet | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    api
      .getResults(campaignId, question.id)
      .then(setResult)
      .catch(() => {
        // Pas encore de résultat calculé — état normal, cf. TallyPanel.
      });
  }, [campaignId, question.id]);

  const options =
    "options" in question.question_type
      ? (question.question_type.options as { id: string; labels: Record<string, string> }[])
      : [];
  const optionLabel = (optionId: string) =>
    options.find((o) => o.id === optionId)?.labels.fr ?? optionId;

  async function handleTally() {
    setError(null);
    setLoading(true);
    try {
      const outcome = await api.tally(campaignId, question.id, {
        voting_method_id: "voting.majority_judgment",
        seats: options.length || 1,
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
        <h3 className="font-medium text-ink">{question.prompt.fr}</h3>
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
        <div className="mt-5 flex items-center justify-between border-t border-line pt-4">
          <p className="text-xs text-muted">Jugement majoritaire — médiane des mentions par option.</p>
          <Button onClick={handleTally} disabled={loading}>
            {loading ? "Calcul…" : "Dépouiller"}
          </Button>
        </div>
      )}

      {result && (
        <div className="mt-5 border-t border-line pt-4">
          <p className="text-xs text-muted">
            {result.outcome.valid_ballots} suffrage(s) exprimé(s) sur {result.outcome.total_ballots}
          </p>
          <div className="mt-3">
            <MajorityJudgmentResults result={result} optionLabel={optionLabel} />
          </div>
        </div>
      )}
    </Card>
  );
}
