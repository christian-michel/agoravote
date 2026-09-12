import { useState } from "react";
import { api, ApiError, type CreateQuestionRequest } from "../api/client";
import { Alert, Button, Card, Field } from "./ui";

interface DraftOption {
  id: string;
  label: string;
}

interface DraftQuestion {
  prompt: string;
  type: "single_choice" | "multiple_choice";
  options: DraftOption[];
}

function emptyOption(): DraftOption {
  return { id: "", label: "" };
}

function emptyQuestion(): DraftQuestion {
  return { prompt: "", type: "single_choice", options: [emptyOption(), emptyOption()] };
}

/**
 * Le constructeur ne couvre que `single_choice`/`multiple_choice`
 * pour l'instant — les autres types déjà supportés par l'API (texte,
 * nombre, échelle, classement, cf. docs/openapi.yaml) demanderont une
 * UI dédiée par type ; ceux-ci couvrent le parcours de démonstration
 * du README backend et la grande majorité des scrutins réels.
 */
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

  function addQuestion() {
    setQuestions((qs) => [...qs, emptyQuestion()]);
  }

  function removeQuestion(index: number) {
    setQuestions((qs) => qs.filter((_, i) => i !== index));
  }

  /** Dérive un identifiant d'option stable à partir du libellé si
   * l'organisateur n'en a pas saisi un explicitement — évite d'exiger
   * un "id technique" en plus du texte affiché, tout en gardant des
   * identifiants distincts d'un simple index (qui casserait si les
   * options sont réordonnées plus tard). */
  function slugify(label: string, fallbackIndex: number): string {
    const slug = label
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_+|_+$/g, "");
    return slug || `option_${fallbackIndex}`;
  }

  async function handleSubmit() {
    setError(null);

    for (const q of questions) {
      if (!q.prompt.trim()) {
        setError("Chaque question doit avoir un intitulé.");
        return;
      }
      if (q.options.filter((o) => o.label.trim()).length < 2) {
        setError(`La question « ${q.prompt} » doit avoir au moins deux options.`);
        return;
      }
    }

    const payload: CreateQuestionRequest[] = questions.map((q) => ({
      prompt: q.prompt.trim(),
      type: q.type,
      required: true,
      options: q.options
        .filter((o) => o.label.trim())
        .map((o, i) => ({
          id: o.id.trim() || slugify(o.label, i),
          labels: { fr: o.label.trim() },
        })),
    }));

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
    <Card>
      <h2 className="font-display text-lg font-medium text-ink">Construire le formulaire</h2>
      <p className="mt-1 text-sm text-muted">
        Une campagne ne peut être publiée qu'une fois son formulaire créé.
      </p>

      {error && (
        <div className="mt-4">
          <Alert>{error}</Alert>
        </div>
      )}

      <div className="mt-6 flex flex-col gap-6">
        {questions.map((question, qIndex) => (
          <div key={qIndex} className="rounded-md border border-line p-4">
            <div className="flex items-start justify-between gap-4">
              <div className="flex-1">
                <Field
                  label={`Question ${qIndex + 1}`}
                  value={question.prompt}
                  onChange={(e) => updateQuestion(qIndex, { prompt: e.target.value })}
                  placeholder="Quelle est votre priorité pour cette année ?"
                />
              </div>
              {questions.length > 1 && (
                <button
                  type="button"
                  onClick={() => removeQuestion(qIndex)}
                  className="mt-7 text-xs text-danger hover:underline"
                >
                  Retirer
                </button>
              )}
            </div>

            <div className="mt-3 flex items-center gap-4 text-sm">
              <label className="flex items-center gap-2">
                <input
                  type="radio"
                  name={`type-${qIndex}`}
                  checked={question.type === "single_choice"}
                  onChange={() => updateQuestion(qIndex, { type: "single_choice" })}
                />
                Choix unique
              </label>
              <label className="flex items-center gap-2">
                <input
                  type="radio"
                  name={`type-${qIndex}`}
                  checked={question.type === "multiple_choice"}
                  onChange={() => updateQuestion(qIndex, { type: "multiple_choice" })}
                />
                Choix multiple
              </label>
            </div>

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
            </div>
          </div>
        ))}

        <button
          type="button"
          onClick={addQuestion}
          className="self-start text-sm text-accent hover:underline"
        >
          + Ajouter une question
        </button>

        <Button onClick={handleSubmit} disabled={submitting}>
          {submitting ? "Création…" : "Créer le formulaire"}
        </Button>
      </div>
    </Card>
  );
}
