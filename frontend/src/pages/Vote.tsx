import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { api, ApiError, type Campaign, type Question } from "../api/client";
import { Alert, Button, Card } from "../components/ui";
import { MAJORITY_JUDGMENT_MENTIONS } from "../lib/majorityJudgmentMentions";

export default function Vote() {
  const { campaignId, questionId } = useParams<{ campaignId: string; questionId: string }>();
  const [campaign, setCampaign] = useState<Campaign | null>(null);
  const [question, setQuestion] = useState<Question | null>(null);
  const [questionCount, setQuestionCount] = useState(1);
  const [questionPosition, setQuestionPosition] = useState(1);

  // Choix unique/multiple : ids sélectionnés (un seul pour choix
  // unique). Classement : mêmes ids, mais l'ORDRE d'ajout porte le
  // rang de préférence (cf. `toggleRanked` ci-dessous) — c'est
  // pourquoi ranking réutilise `selected` plutôt qu'un état séparé.
  const [selected, setSelected] = useState<string[]>([]);
  const [textValue, setTextValue] = useState("");
  const [numericValue, setNumericValue] = useState<number | null>(null);
  // Jugement majoritaire : une mention (1-6) par id d'option — cf.
  // `agoravote_core::QuestionType::MajorityJudgment`.
  const [mentions, setMentions] = useState<Record<string, number>>({});

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

  /** Classement : cliquer une option non classée l'ajoute à la fin de
   * l'ordre de préférence (rang = position dans `selected`) ; cliquer
   * une option déjà classée la retire, décalant les rangs suivants —
   * pas de glisser-déposer (cf. DESIGN.md, cohérent avec le reste de
   * l'app qui n'utilise aucune bibliothèque d'interaction complexe). */
  function toggleRanked(optionId: string) {
    setSelected((current) =>
      current.includes(optionId)
        ? current.filter((id) => id !== optionId)
        : [...current, optionId],
    );
  }

  async function handleSubmit() {
    if (!campaignId || !questionId || !question) return;
    const type = question.question_type.type;
    const body: {
      selections?: string[];
      scores?: Record<string, number>;
      numeric_value?: number;
      text_value?: string;
    } = {};
    if (type === "single_choice" || type === "multiple_choice" || type === "ranking") {
      if (selected.length === 0) return;
      body.selections = selected;
    } else if (type === "number" || type === "scale") {
      if (numericValue === null) return;
      body.numeric_value = numericValue;
    } else if (type === "text") {
      if (textValue.trim().length === 0) return;
      body.text_value = textValue;
    } else if (type === "majority_judgment") {
      if (Object.keys(mentions).length === 0) return;
      body.scores = mentions;
    }

    setSubmitting(true);
    setError(null);
    try {
      await api.castBallot(campaignId, questionId, body);
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
        <h1 className="text-2xl font-semibold text-ink">Vote enregistré</h1>
        <p className="mt-3 text-sm text-ink-soft">Merci pour votre participation.</p>
      </div>
    );
  }

  if (!question) return null;

  const type = question.question_type.type;
  const percent = Math.round((questionPosition / questionCount) * 100);

  let canSubmit = false;
  if (type === "single_choice" || type === "multiple_choice") {
    canSubmit = selected.length > 0;
  } else if (type === "ranking") {
    canSubmit =
      "options" in question.question_type &&
      selected.length === question.question_type.options.length;
  } else if (type === "number" || type === "scale") {
    canSubmit = numericValue !== null;
  } else if (type === "text") {
    canSubmit = textValue.trim().length > 0;
  } else if (type === "majority_judgment") {
    canSubmit =
      "options" in question.question_type &&
      question.question_type.options.every((o) => mentions[o.id] !== undefined);
  }

  return (
    <div className="mx-auto grid max-w-3xl gap-8 lg:grid-cols-[1.3fr_1fr] lg:items-center">
      <div>
        <div className="flex items-center justify-between text-xs font-medium uppercase tracking-wide text-muted">
          <span>
            Question {questionPosition} sur {questionCount}
          </span>
          <span>{percent}%</span>
        </div>
        <div className="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-line/60">
          <div
            className="h-full bg-accent transition-all"
            style={{ width: `${percent}%` }}
          />
        </div>

        <h1 className="mt-6 text-2xl font-semibold leading-snug text-ink">
          {question.prompt.fr}
        </h1>

        <Card className="mt-6">
          {(type === "single_choice" || type === "multiple_choice") &&
            "options" in question.question_type && (
              <fieldset className="flex flex-col gap-3">
                <legend className="sr-only">Options de réponse</legend>
                {question.question_type.options.map((option) => (
                  <label
                    key={option.id}
                    className="flex cursor-pointer items-center gap-3 rounded-md border border-line px-4 py-3 text-sm has-checked:border-accent has-checked:bg-accent-soft"
                  >
                    <input
                      type={type === "multiple_choice" ? "checkbox" : "radio"}
                      name="option"
                      checked={selected.includes(option.id)}
                      onChange={() => toggleOption(option.id, type === "multiple_choice")}
                    />
                    <span className="text-ink">{option.labels.fr}</span>
                  </label>
                ))}
              </fieldset>
            )}

          {type === "ranking" && "options" in question.question_type && (
            <fieldset className="flex flex-col gap-3">
              <legend className="text-xs text-muted">
                Cliquez les options dans votre ordre de préférence (1 = préférée).
              </legend>
              {question.question_type.options.map((option) => {
                const rank = selected.indexOf(option.id);
                return (
                  <button
                    key={option.id}
                    type="button"
                    onClick={() => toggleRanked(option.id)}
                    className={`flex items-center gap-3 rounded-md border px-4 py-3 text-left text-sm ${
                      rank >= 0 ? "border-accent bg-accent-soft" : "border-line"
                    }`}
                  >
                    <span
                      className={`flex h-6 w-6 shrink-0 items-center justify-center rounded-full text-xs font-semibold ${
                        rank >= 0 ? "bg-accent text-white" : "bg-line/60 text-muted"
                      }`}
                    >
                      {rank >= 0 ? rank + 1 : ""}
                    </span>
                    <span className="text-ink">{option.labels.fr}</span>
                  </button>
                );
              })}
            </fieldset>
          )}

          {type === "scale" &&
            "min" in question.question_type &&
            (() => {
              // Destructurés en variables locales AVANT le callback
              // `Array.from` : TypeScript ne propage pas le
              // rétrécissement de type d'une union discriminée dans
              // une fonction imbriquée — cf. l'erreur de compilation
              // rencontrée en écrivant ce fichier, résolue ainsi.
              const { min, max } = question.question_type as { min: number; max: number };
              return (
                <div className="flex flex-col gap-3">
                  <div className="flex flex-wrap justify-center gap-2">
                    {Array.from({ length: max - min + 1 }, (_, i) => min + i).map((value) => (
                      <button
                        key={value}
                        type="button"
                        onClick={() => setNumericValue(value)}
                        className={`flex h-11 w-11 items-center justify-center rounded-full border text-sm font-medium ${
                          numericValue === value
                            ? "border-accent bg-accent text-white"
                            : "border-line text-ink hover:border-accent/50"
                        }`}
                      >
                        {value}
                      </button>
                    ))}
                  </div>
                  <div className="flex justify-between text-xs text-muted">
                    <span>{min}</span>
                    <span>{max}</span>
                  </div>
                </div>
              );
            })()}

          {type === "number" &&
            (() => {
              const bounds =
                "min" in question.question_type
                  ? (question.question_type as { min?: number | null; max?: number | null })
                  : {};
              return (
                <div className="flex flex-col gap-1.5">
                  <input
                    type="number"
                    className="w-full rounded-md border border-line bg-surface px-3 py-2.5 text-sm text-ink"
                    value={numericValue ?? ""}
                    min={bounds.min ?? undefined}
                    max={bounds.max ?? undefined}
                    onChange={(e) =>
                      setNumericValue(e.target.value === "" ? null : Number(e.target.value))
                    }
                    placeholder="Votre réponse"
                  />
                  {(bounds.min != null || bounds.max != null) && (
                    <p className="text-xs text-muted">
                      {bounds.min != null && bounds.max != null
                        ? `Entre ${bounds.min} et ${bounds.max}`
                        : bounds.min != null
                          ? `Minimum ${bounds.min}`
                          : `Maximum ${bounds.max}`}
                    </p>
                  )}
                </div>
              );
            })()}

          {type === "text" && (
            <textarea
              className="w-full rounded-md border border-line bg-surface px-3 py-2.5 text-sm text-ink"
              rows={4}
              value={textValue}
              maxLength={
                "max_length" in question.question_type
                  ? ((question.question_type as { max_length?: number | null }).max_length ??
                    undefined)
                  : undefined
              }
              onChange={(e) => setTextValue(e.target.value)}
              placeholder="Votre réponse"
            />
          )}

          {type === "majority_judgment" && "options" in question.question_type && (
            <fieldset className="flex flex-col gap-4">
              <legend className="text-xs text-muted">
                Attribuez une mention à chaque proposition.
              </legend>
              {question.question_type.options.map((option) => (
                <div key={option.id} className="flex flex-col gap-2">
                  <span className="text-sm font-medium text-ink">{option.labels.fr}</span>
                  <div className="flex flex-wrap gap-1.5">
                    {MAJORITY_JUDGMENT_MENTIONS.map((mention) => {
                      const selected = mentions[option.id] === mention.value;
                      return (
                        <button
                          key={mention.value}
                          type="button"
                          onClick={() =>
                            setMentions((current) => ({ ...current, [option.id]: mention.value }))
                          }
                          className="rounded-md border-2 px-2.5 py-1.5 text-xs font-medium transition-transform"
                          style={{
                            borderColor: mention.color,
                            backgroundColor: selected ? mention.color : "transparent",
                            color: selected ? "#fff" : "var(--color-ink)",
                            transform: selected ? "translateY(-1px)" : undefined,
                          }}
                        >
                          {mention.label}
                        </button>
                      );
                    })}
                  </div>
                </div>
              ))}
            </fieldset>
          )}

          {error && (
            <div className="mt-4">
              <Alert>{error}</Alert>
            </div>
          )}

          <Button
            className="mt-6 w-full"
            onClick={handleSubmit}
            disabled={!canSubmit || submitting}
          >
            {submitting ? "Envoi…" : "Voter"}
          </Button>
        </Card>
      </div>

      <Card className="hidden bg-accent-soft text-center lg:block">
        <p className="text-4xl">🗳️</p>
        <p className="mt-4 font-medium text-ink">Votre participation est importante !</p>
        <p className="mt-2 text-sm text-ink-soft">
          Cette consultation utilise une méthode de calcul transparente — le résultat
          sera expliqué, pas seulement affiché.
        </p>
      </Card>
    </div>
  );
}
