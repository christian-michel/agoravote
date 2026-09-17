import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, type Question, type QuestionResponses } from "../api/client";
import { useLanguage } from "../i18n/LanguageContext";
import { resolveLocalizedText } from "../i18n/content";
import { Card } from "./ui";
import { ResponsesView } from "./ResponsesView";

/**
 * Pendant de `TallyPanel` pour les types de question qu'aucune
 * méthode de vote ne dépouille (Texte, Nombre, Échelle, Classement) —
 * cf. `dto::QuestionResponses` côté API. Toujours "en direct" : pas de
 * bouton "Dépouiller" à afficher, contrairement à `TallyPanel`, il n'y
 * a rien à calculer qui nécessite de choisir une méthode.
 */
export function ResponsesPanel({
  campaignId,
  question,
}: {
  campaignId: string;
  question: Question;
}) {
  const { t, lang } = useLanguage();
  const [responses, setResponses] = useState<QuestionResponses | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    api
      .getResponses(campaignId, question.id)
      .then(setResponses)
      .finally(() => setLoading(false));
  }, [campaignId, question.id]);

  return (
    <Card>
      <div className="flex items-start justify-between gap-4">
        <h3 className="font-medium text-ink">{resolveLocalizedText(question.prompt, lang)}</h3>
        <Link
          to={`/campagnes/${campaignId}/questions/${question.id}/resultats`}
          className="shrink-0 text-xs text-accent hover:underline"
        >
          {t("tally.publicResultsLink")}
        </Link>
      </div>
      <div className="mt-4 border-t border-line pt-4">
        {loading ? (
          <p className="text-sm text-muted">{t("common.loading")}</p>
        ) : responses ? (
          <ResponsesView responses={responses} questionType={question.question_type} />
        ) : (
          <p className="text-sm text-muted">{t("responses.loadError")}</p>
        )}
      </div>
    </Card>
  );
}
