import { useState } from "react";
import { api, ApiError, type CreateQuestionRequest } from "../api/client";
import { Alert, Button, Card, Field } from "./ui";
import { TrashIcon } from "./icons";

type QuestionKind = "single_choice" | "multiple_choice" | "text" | "number" | "scale" | "ranking";

interface DraftOption {
  id: string;
  label: string;
}

interface DraftQuestion {
  prompt: string;
  type: QuestionKind;
  options: DraftOption[];
  maxSelections: string;
  maxLength: string;
  min: string;
  max: string;
}

/** Bibliothèque de types de question — les six types déjà acceptés par
 * l'API (cf. docs/openapi.yaml, `QuestionType`), tous exposés ici pour
 * la première fois : auparavant seuls choix unique/multiple avaient
 * une UI (cf. docs/ROADMAP.md, "types de question supplémentaires").
 */
const QUESTION_LIBRARY: { type: QuestionKind; label: string; needsOptions: boolean }[] = [
  { type: "single_choice", label: "Choix unique", needsOptions: true },
  { type: "multiple_choice", label: "Choix multiple", needsOptions: true },
  { type: "ranking", label: "Classement", needsOptions: true },
  { type: "text", label: "Texte libre", needsOptions: false },
  { type: "number", label: "Nombre", needsOptions: false },
  { type: "scale", label: "Échelle", needsOptions: false },
];

function emptyOption(): DraftOption {
  return { id: "", label: "" };
}

function emptyQuestion(type: QuestionKind = "single_choice"): DraftQuestion {
  const needsOptions = QUESTION_LIBRARY.find((q) => q.type === type)?.needsOptions ?? false;
  return {
    prompt: "",
    type,
    options: needsOptions ? [emptyOption(), emptyOption()] : [],
    maxSelections: "",
    maxLength: "",
    min: type === "scale" ? "1" : "",
    max: type === "scale" ? "5" : "",
  };
}

function slugify(label: string, fallbackIndex: number): string {
  const slug = label
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
  return slug || `option_${fallbackIndex}`;
}

export function FormBuilder({
  campaignId,
  onCreated,
}: {
  campaignId: string;
  onCreated: () => void;
}) {
  const [questions, setQuestions] = useState<DraftQuestion[]>([emptyQuestion()]);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  function updateQuestion(index: number, patch: Partial<DraftQuestion>) {
    setQuestions((qs) => qs.map((q, i) => (i === index ? { ...q, ...patch } : q)));
  }

  function updateOption(qIndex: number, oIndex: number, patch: Partial<DraftOption>) {
    setQuestions((qs) =>
      qs.map((q, i) =>
        i === qIndex
          ? { ...q, options: q.options.map((o, j) => (j === oIndex ? { ...o, ...patch } : o)) }
          : q,
      ),
    );
  }

  function addOption(qIndex: number) {
    setQuestions((qs) =>
      qs.map((q, i) => (i === qIndex ? { ...q, options: [...q.options, emptyOption()] } : q)),
    );
  }

  function removeOption(qIndex: number, oIndex: number) {
    setQuestions((qs) =>
      qs.map((q, i) =>
        i === qIndex ? { ...q, options: q.options.filter((_, j) => j !== oIndex) } : q,
      ),
    );
  }

  function addQuestion(type: QuestionKind) {
    setQuestions((qs) => [...qs, emptyQuestion(type)]);
  }

  function removeQuestion(index: number) {
    setQuestions((qs) => qs.filter((_, i) => i !== index));
  }

  function moveQuestion(index: number, direction: -1 | 1) {
    setQuestions((qs) => {
      const target = index + direction;
      if (target < 0 || target >= qs.length) return qs;
      const copy = [...qs];
      [copy[index], copy[target]] = [copy[target], copy[index]];
      return copy;
    });
  }

  async function handleSubmit() {
    setError(null);

    for (const q of questions) {
      if (!q.prompt.trim()) {
        setError("Chaque question doit avoir un intitulé.");
        return;
      }
      const needsOptions = QUESTION_LIBRARY.find((item) => item.type === q.type)?.needsOptions;
      if (needsOptions && q.options.filter((o) => o.label.trim()).length < 2) {
        setError(`La question « ${q.prompt} » doit avoir au moins deux options.`);
        return;
      }
      if (q.type === "scale" && (q.min.trim() === "" || q.max.trim() === "")) {
        setError(`La question « ${q.prompt} » (échelle) doit avoir un minimum et un maximum.`);
        return;
      }
    }

    const payload: CreateQuestionRequest[] = questions.map((q) => {
      const options = q.options
        .filter((o) => o.label.trim())
        .map((o, i) => ({
          id: o.id.trim() || slugify(o.label, i),
          labels: { fr: o.label.trim() },
        }));

      const base = { prompt: q.prompt.trim(), required: true as const };
      switch (q.type) {
        case "single_choice":
          return { ...base, type: "single_choice", options };
        case "multiple_choice":
          return {
            ...base,
            type: "multiple_choice",
            options,
            max_selections: q.maxSelections ? Number(q.maxSelections) : undefined,
          };
        case "ranking":
          return { ...base, type: "ranking", options };
        case "text":
          return { ...base, type: "text", max_length: q.maxLength ? Number(q.maxLength) : undefined };
        case "number":
          return {
            ...base,
            type: "number",
            min: q.min ? Number(q.min) : undefined,
            max: q.max ? Number(q.max) : undefined,
          };
        case "scale":
          return { ...base, type: "scale", min: Number(q.min), max: Number(q.max) };
      }
    });

    setSubmitting(true);
    try {
      await api.createForm(campaignId, { questions: payload });
      onCreated();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "La création du formulaire a échoué.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="grid gap-6 lg:grid-cols-[220px_1fr]">
      <Card className="h-fit">
        <h2 className="text-sm font-medium text-ink">Bibliothèque de questions</h2>
        <p className="mt-1 text-xs text-muted">Cliquez pour ajouter au formulaire.</p>
        <div className="mt-4 flex flex-col gap-2">
          {QUESTION_LIBRARY.map((item) => (
            <button
              key={item.type}
              type="button"
              onClick={() => addQuestion(item.type)}
              className="rounded-md border border-line px-3 py-2 text-left text-sm text-ink-soft transition-colors hover:border-accent hover:text-ink"
            >
              {item.label}
            </button>
          ))}
        </div>
      </Card>

      <Card>
        <h2 className="font-medium text-ink">Construire le formulaire</h2>
        <p className="mt-1 text-sm text-muted">
          Une campagne ne peut être publiée qu'une fois son formulaire créé.
        </p>

        {error && (
          <div className="mt-4">
            <Alert>{error}</Alert>
          </div>
        )}

        <div className="mt-6 flex flex-col gap-6">
          {questions.map((question, qIndex) => {
            const libraryItem = QUESTION_LIBRARY.find((item) => item.type === question.type);
            return (
              <div key={qIndex} className="rounded-lg border border-line p-4">
                <div className="flex items-start justify-between gap-4">
                  <div className="flex items-center gap-2">
                    <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-accent-soft text-xs font-semibold text-accent-strong">
                      {qIndex + 1}
                    </span>
                    <span className="rounded-full bg-paper px-2 py-0.5 text-xs text-muted">
                      {libraryItem?.label}
                    </span>
                  </div>
                  <div className="flex items-center gap-3 text-xs">
                    <button
                      type="button"
                      onClick={() => moveQuestion(qIndex, -1)}
                      disabled={qIndex === 0}
                      className="text-muted hover:text-ink disabled:opacity-30"
                      aria-label="Monter"
                    >
                      ↑
                    </button>
                    <button
                      type="button"
                      onClick={() => moveQuestion(qIndex, 1)}
                      disabled={qIndex === questions.length - 1}
                      className="text-muted hover:text-ink disabled:opacity-30"
                      aria-label="Descendre"
                    >
                      ↓
                    </button>
                    {questions.length > 1 && (
                      <button
                        type="button"
                        onClick={() => removeQuestion(qIndex)}
                        className="text-danger hover:opacity-70"
                        aria-label="Retirer la question"
                      >
                        <TrashIcon width={16} height={16} />
                      </button>
                    )}
                  </div>
                </div>

                <div className="mt-3">
                  <Field
                    label="Intitulé de la question"
                    value={question.prompt}
                    onChange={(e) => updateQuestion(qIndex, { prompt: e.target.value })}
                    placeholder="Quelle est votre priorité pour cette année ?"
                  />
                </div>

                {libraryItem?.needsOptions && (
                  <div className="mt-4 flex flex-col gap-2">
                    {question.options.map((option, oIndex) => (
                      <div key={oIndex} className="flex items-center gap-2">
                        <input
                          className="flex-1 rounded-md border border-line bg-surface px-3 py-1.5 text-sm"
                          placeholder={`Option ${oIndex + 1}`}
                          value={option.label}
                          onChange={(e) => updateOption(qIndex, oIndex, { label: e.target.value })}
                        />
                        {question.options.length > 2 && (
                          <button
                            type="button"
                            onClick={() => removeOption(qIndex, oIndex)}
                            className="text-xs text-muted hover:text-danger"
                            aria-label="Retirer cette option"
                          >
                            ✕
                          </button>
                        )}
                      </div>
                    ))}
                    <button
                      type="button"
                      onClick={() => addOption(qIndex)}
                      className="self-start text-xs text-accent hover:underline"
                    >
                      + Ajouter une option
                    </button>
                    {question.type === "multiple_choice" && (
                      <input
                        className="mt-1 w-48 rounded-md border border-line bg-surface px-3 py-1.5 text-xs"
                        type="number"
                        min={1}
                        placeholder="Max. sélections (optionnel)"
                        value={question.maxSelections}
                        onChange={(e) => updateQuestion(qIndex, { maxSelections: e.target.value })}
                      />
                    )}
                  </div>
                )}

                {question.type === "text" && (
                  <input
                    className="mt-3 w-56 rounded-md border border-line bg-surface px-3 py-1.5 text-xs"
                    type="number"
                    min={1}
                    placeholder="Longueur max. (optionnel)"
                    value={question.maxLength}
                    onChange={(e) => updateQuestion(qIndex, { maxLength: e.target.value })}
                  />
                )}

                {question.type === "number" && (
                  <div className="mt-3 flex gap-2">
                    <input
                      className="w-28 rounded-md border border-line bg-surface px-3 py-1.5 text-xs"
                      type="number"
                      placeholder="Min (optionnel)"
                      value={question.min}
                      onChange={(e) => updateQuestion(qIndex, { min: e.target.value })}
                    />
                    <input
                      className="w-28 rounded-md border border-line bg-surface px-3 py-1.5 text-xs"
                      type="number"
                      placeholder="Max (optionnel)"
                      value={question.max}
                      onChange={(e) => updateQuestion(qIndex, { max: e.target.value })}
                    />
                  </div>
                )}

                {question.type === "scale" && (
                  <div className="mt-3 flex gap-2">
                    <input
                      className="w-28 rounded-md border border-line bg-surface px-3 py-1.5 text-xs"
                      type="number"
                      placeholder="Minimum"
                      value={question.min}
                      onChange={(e) => updateQuestion(qIndex, { min: e.target.value })}
                    />
                    <input
                      className="w-28 rounded-md border border-line bg-surface px-3 py-1.5 text-xs"
                      type="number"
                      placeholder="Maximum"
                      value={question.max}
                      onChange={(e) => updateQuestion(qIndex, { max: e.target.value })}
                    />
                  </div>
                )}
              </div>
            );
          })}

          <Button onClick={handleSubmit} disabled={submitting}>
            {submitting ? "Création…" : "Créer le formulaire"}
          </Button>
        </div>
      </Card>
    </div>
  );
}
