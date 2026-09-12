import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { api, ApiError, type Campaign, type Question } from "../api/client";
import { Alert, Button, Card } from "../components/ui";

export default function Vote() {
  const { campaignId, questionId } = useParams<{ campaignId: string; questionId: string }>();
  const [campaign, setCampaign] = useState<Campaign | null>(null);
  const [question, setQuestion] = useState<Question | null>(null);
  const [questionCount, setQuestionCount] = useState(1);
  const [questionPosition, setQuestionPosition] = useState(1);
  const [selected, setSelected] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  useEffect(() => {
    if (!campaignId || !questionId) return;
    (async () => {
      setLoading(true);
      setError(null);
      try {
        const [c, f] = await Promise.all([
          api.getCampaign(campaignId),
          api.getForm(campaignId),
        ]);
        setCampaign(c);
        const q = f.questions.find((item) => item.id === questionId);
        if (!q) {
          setError("Cette question n'existe pas dans cette campagne.");
          return;
        }
        setQuestion(q);
        setQuestionCount(f.questions.length);
        setQuestionPosition(f.questions.findIndex((item) => item.id === questionId) + 1);
      } catch (err) {
        setError(err instanceof ApiError ? err.message : "Impossible de charger cette question.");
      } finally {
        setLoading(false);
      }
    })();
  }, [campaignId, questionId]);

  function toggleOption(optionId: string, multiple: boolean) {
    setSelected((current) => {
      if (multiple) {
        return current.includes(optionId)
          ? current.filter((id) => id !== optionId)
          : [...current, optionId];
      }
      return [optionId];
    });
  }

  async function handleSubmit() {
    if (!campaignId || !questionId || selected.length === 0) return;
    setSubmitting(true);
    setError(null);
    try {
      await api.castBallot(campaignId, questionId, { selections: selected });
      setSubmitted(true);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Le vote n'a pas pu être enregistré.");
    } finally {
      setSubmitting(false);
    }
  }

  if (loading) return <p className="text-sm text-muted">Chargement…</p>;

  if (error) {
    return (
      <div className="mx-auto max-w-md">
        <Alert>{error}</Alert>
      </div>
    );
  }

  if (campaign && campaign.status !== "Published") {
    return (
      <div className="mx-auto max-w-md text-center">
        <p className="text-sm text-ink-soft">
          Cette campagne n'accepte pas de vote pour le moment
          {campaign.status === "Draft" ? " (elle n'a pas encore été publiée)." : " (elle est clôturée)."}
        </p>
        {campaign.status === "Closed" && questionId && campaignId && (
          <Link
            to={`/campagnes/${campaignId}/questions/${questionId}/resultats`}
            className="mt-4 inline-block text-sm text-accent hover:underline"
          >
            Voir les résultats →
          </Link>
        )}
      </div>
    );
  }

  if (submitted) {
    return (
      <div className="mx-auto max-w-md text-center">
        <h1 className="font-display text-2xl font-medium text-ink">Vote enregistré</h1>
        <p className="mt-3 text-sm text-ink-soft">Merci pour votre participation.</p>
      </div>
    );
  }

  if (!question) return null;

  const multiple = "options" in question.question_type && question.question_type.type === "multiple_choice";
  const options =
    "options" in question.question_type ? (question.question_type.options as { id: string; labels: Record<string, string> }[]) : [];

  return (
    <div className="mx-auto max-w-lg">
      <p className="text-xs font-medium uppercase tracking-wide text-muted">
        Question {questionPosition} sur {questionCount}
      </p>
      <div className="mt-2 h-1 w-full overflow-hidden rounded-full bg-line/60">
        <div
          className="h-full bg-accent transition-all"
          style={{ width: `${(questionPosition / questionCount) * 100}%` }}
        />
      </div>

      <h1 className="mt-6 font-display text-2xl font-medium leading-snug text-ink">
        {question.prompt.fr}
      </h1>

      <Card className="mt-6">
        <fieldset className="flex flex-col gap-3">
          <legend className="sr-only">Options de réponse</legend>
          {options.map((option) => (
            <label
              key={option.id}
              className="flex cursor-pointer items-center gap-3 rounded-md border border-line px-4 py-3 text-sm has-checked:border-accent has-checked:bg-accent-soft"
            >
              <input
                type={multiple ? "checkbox" : "radio"}
                name="option"
                checked={selected.includes(option.id)}
                onChange={() => toggleOption(option.id, multiple)}
              />
              <span className="text-ink">{option.labels.fr}</span>
            </label>
          ))}
        </fieldset>

        {error && (
          <div className="mt-4">
            <Alert>{error}</Alert>
          </div>
        )}

        <Button
          className="mt-6 w-full"
          onClick={handleSubmit}
          disabled={selected.length === 0 || submitting}
        >
          {submitting ? "Envoi…" : "Voter"}
        </Button>
      </Card>
    </div>
  );
}
