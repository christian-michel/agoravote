import { useState } from "react";
import { api, ApiError, type CreateQuestionRequest } from "../api/client";
import { useLanguage } from "../i18n/LanguageContext";
import type { TranslationKey } from "../i18n/translations/fr";
import { Alert, Button, Card, Field } from "./ui";
import { TrashIcon } from "./icons";

type QuestionKind =
  | "single_choice"
  | "multiple_choice"
  | "text"
  | "number"
  | "scale"
  | "ranking"
  | "majority_judgment";

interface DraftOption {
  id: string;
  label: string;
  labelEn: string;
}

interface DraftQuestion {
  prompt: string;
  promptEn: string;
  type: QuestionKind;
  options: DraftOption[];
  maxSelections: string;
  maxLength: string;
  min: string;
  max: string;
}

/** Bibliothèque de types de question — les sept types déjà acceptés
 * par l'API (cf. docs/openapi.yaml, `QuestionType`). Le libellé de
 * chaque type est résolu via `t()` au moment de l'affichage (cf.
 * `QUESTION_LIBRARY_LABEL_KEYS` ci-dessous), pas stocké tel quel ici :
 * cette table ne connaît que la structure de saisie, pas la langue.
 */
const QUESTION_LIBRARY: { type: QuestionKind; needsOptions: boolean }[] = [
  { type: "single_choice", needsOptions: true },
  { type: "multiple_choice", needsOptions: true },
  { type: "ranking", needsOptions: true },
  { type: "majority_judgment", needsOptions: true },
  { type: "text", needsOptions: false },
  { type: "number", needsOptions: false },
  { type: "scale", needsOptions: false },
];

const QUESTION_LIBRARY_LABEL_KEYS: Record<QuestionKind, TranslationKey> = {
  single_choice: "formBuilder.typeSingleChoice",
  multiple_choice: "formBuilder.typeMultipleChoice",
  ranking: "formBuilder.typeRanking",
  majority_judgment: "formBuilder.typeMajorityJudgment",
  text: "formBuilder.typeText",
  number: "formBuilder.typeNumber",
  scale: "formBuilder.typeScale",
};

function emptyOption(): DraftOption {
  return { id: "", label: "", labelEn: "" };
}

function emptyQuestion(type: QuestionKind = "single_choice"): DraftQuestion {
  const needsOptions = QUESTION_LIBRARY.find((q) => q.type === type)?.needsOptions ?? false;
  return {
    prompt: "",
    promptEn: "",
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
    .replace(/[̀-ͯ]/g, "")
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
  return slug || `option_${fallbackIndex}`;
}

/** Construit la table langue -> texte d'un champ traduisible : "fr"
 * toujours présent (requis côté API, cf. `routes.rs::create_form`),
 * "en" seulement si l'organisateur a saisi une traduction — un champ
 * anglais vide ne produit pas de clé "en" vide plutôt que de laisser
 * l'affichage retomber sur une chaîne vide au lieu du repli français
 * (cf. `resolveLocalizedText`). */
function localizedRecord(fr: string, en: string): Record<string, string> {
  const record: Record<string, string> = { fr: fr.trim() };
  if (en.trim()) record.en = en.trim();
  return record;
}

export function FormBuilder({
  campaignId,
  onCreated,
}: {
  campaignId: string;
  onCreated: () => void;
}) {
  const { t } = useLanguage();
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
        setError(t("formBuilder.errorPromptRequired"));
        return;
      }
      const needsOptions = QUESTION_LIBRARY.find((item) => item.type === q.type)?.needsOptions;
      if (needsOptions && q.options.filter((o) => o.label.trim()).length < 2) {
        setError(t("formBuilder.errorNeedsOptions", { prompt: q.prompt }));
        return;
      }
      if (q.type === "scale" && (q.min.trim() === "" || q.max.trim() === "")) {
        setError(t("formBuilder.errorScaleBounds", { prompt: q.prompt }));
        return;
      }
    }

    const payload: CreateQuestionRequest[] = questions.map((q) => {
      const options = q.options
        .filter((o) => o.label.trim())
        .map((o, i) => ({
          id: o.id.trim() || slugify(o.label, i),
          labels: localizedRecord(o.label, o.labelEn),
        }));

      const base = { prompt: localizedRecord(q.prompt, q.promptEn), required: true as const };
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
        case "majority_judgment":
          return { ...base, type: "majority_judgment", options };
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
      setError(err instanceof ApiError ? err.message : t("formBuilder.submitError"));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="grid gap-6 lg:grid-cols-[220px_1fr]">
      <Card className="h-fit">
        <h2 className="text-sm font-medium text-ink">{t("formBuilder.libraryTitle")}</h2>
        <p className="mt-1 text-xs text-muted">{t("formBuilder.libraryHint")}</p>
        <div className="mt-4 flex flex-col gap-2">
          {QUESTION_LIBRARY.map((item) => (
            <button
              key={item.type}
              type="button"
              onClick={() => addQuestion(item.type)}
              className="rounded-md border border-line px-3 py-2 text-left text-sm text-ink-soft transition-colors hover:border-accent hover:text-ink"
            >
              {t(QUESTION_LIBRARY_LABEL_KEYS[item.type])}
            </button>
          ))}
        </div>
      </Card>

      <Card>
        <h2 className="font-medium text-ink">{t("formBuilder.buildTitle")}</h2>
        <p className="mt-1 text-sm text-muted">{t("formBuilder.buildHint")}</p>

        {error && (
          <div className="mt-4">
            <Alert>{error}</Alert>
          </div>
        )}

        <div className="mt-6 flex flex-col gap-6">
          {questions.map((question, qIndex) => {
            const needsOptions = QUESTION_LIBRARY.find((item) => item.type === question.type)?.needsOptions;
            return (
              <div key={qIndex} className="rounded-lg border border-line p-4">
                <div className="flex items-start justify-between gap-4">
                  <div className="flex items-center gap-2">
                    <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-accent-soft text-xs font-semibold text-accent-strong">
                      {qIndex + 1}
                    </span>
                    <span className="rounded-full bg-paper px-2 py-0.5 text-xs text-muted">
                      {t(QUESTION_LIBRARY_LABEL_KEYS[question.type])}
                    </span>
                  </div>
                  <div className="flex items-center gap-3 text-xs">
                    <button
                      type="button"
                      onClick={() => moveQuestion(qIndex, -1)}
                      disabled={qIndex === 0}
                      className="text-muted hover:text-ink disabled:opacity-30"
                      aria-label={t("formBuilder.moveUp")}
                    >
                      ↑
                    </button>
                    <button
                      type="button"
                      onClick={() => moveQuestion(qIndex, 1)}
                      disabled={qIndex === questions.length - 1}
                      className="text-muted hover:text-ink disabled:opacity-30"
                      aria-label={t("formBuilder.moveDown")}
                    >
                      ↓
                    </button>
                    {questions.length > 1 && (
                      <button
                        type="button"
                        onClick={() => removeQuestion(qIndex)}
                        className="text-danger hover:opacity-70"
                        aria-label={t("formBuilder.removeQuestion")}
                      >
                        <TrashIcon width={16} height={16} />
                      </button>
                    )}
                  </div>
                </div>

                <div className="mt-3 grid gap-2 sm:grid-cols-2">
                  <Field
                    label={t("formBuilder.promptLabel")}
                    value={question.prompt}
                    onChange={(e) => updateQuestion(qIndex, { prompt: e.target.value })}
                    placeholder={t("formBuilder.promptPlaceholder")}
                  />
                  <Field
                    label={t("formBuilder.promptLabelEn")}
                    value={question.promptEn}
                    onChange={(e) => updateQuestion(qIndex, { promptEn: e.target.value })}
                  />
                </div>

                {needsOptions && (
                  <div className="mt-4 flex flex-col gap-2">
                    {question.options.map((option, oIndex) => (
                      <div key={oIndex} className="flex items-center gap-2">
                        <input
                          className="flex-1 rounded-md border border-line bg-surface px-3 py-1.5 text-sm"
                          placeholder={t("formBuilder.optionPlaceholder", { n: oIndex + 1 })}
                          value={option.label}
                          onChange={(e) => updateOption(qIndex, oIndex, { label: e.target.value })}
                        />
                        <input
                          className="flex-1 rounded-md border border-line bg-surface px-3 py-1.5 text-sm"
                          placeholder={t("formBuilder.optionPlaceholderEn")}
                          value={option.labelEn}
                          onChange={(e) => updateOption(qIndex, oIndex, { labelEn: e.target.value })}
                        />
                        {question.options.length > 2 && (
                          <button
                            type="button"
                            onClick={() => removeOption(qIndex, oIndex)}
                            className="text-xs text-muted hover:text-danger"
                            aria-label={t("formBuilder.removeOption")}
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
                      {t("formBuilder.addOption")}
                    </button>
                    {question.type === "multiple_choice" && (
                      <input
                        className="mt-1 w-48 rounded-md border border-line bg-surface px-3 py-1.5 text-xs"
                        type="number"
                        min={1}
                        placeholder={t("formBuilder.maxSelections")}
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
                    placeholder={t("formBuilder.maxLength")}
                    value={question.maxLength}
                    onChange={(e) => updateQuestion(qIndex, { maxLength: e.target.value })}
                  />
                )}

                {question.type === "number" && (
                  <div className="mt-3 flex gap-2">
                    <input
                      className="w-28 rounded-md border border-line bg-surface px-3 py-1.5 text-xs"
                      type="number"
                      placeholder={t("formBuilder.min")}
                      value={question.min}
                      onChange={(e) => updateQuestion(qIndex, { min: e.target.value })}
                    />
                    <input
                      className="w-28 rounded-md border border-line bg-surface px-3 py-1.5 text-xs"
                      type="number"
                      placeholder={t("formBuilder.max")}
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
                      placeholder={t("formBuilder.scaleMin")}
                      value={question.min}
                      onChange={(e) => updateQuestion(qIndex, { min: e.target.value })}
                    />
                    <input
                      className="w-28 rounded-md border border-line bg-surface px-3 py-1.5 text-xs"
                      type="number"
                      placeholder={t("formBuilder.scaleMax")}
                      value={question.max}
                      onChange={(e) => updateQuestion(qIndex, { max: e.target.value })}
                    />
                  </div>
                )}
              </div>
            );
          })}

          <Button onClick={handleSubmit} disabled={submitting}>
            {submitting ? t("formBuilder.submitting") : t("formBuilder.submit")}
          </Button>
        </div>
      </Card>
    </div>
  );
}
