import { useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { api, ApiError, type Campaign, type Form, type ModuleManifest } from "../api/client";
import { useLanguage, pluralize } from "../i18n/LanguageContext";
import { Alert, Button, Card, StatusBadge } from "../components/ui";
import { FormBuilder } from "../components/FormBuilder";
import { TallyPanel } from "../components/TallyPanel";
import { ResponsesPanel } from "../components/ResponsesPanel";
import { MajorityJudgmentPanel } from "../components/MajorityJudgmentPanel";
import { rememberCampaign } from "../lib/recentCampaigns";

export default function CampaignManage() {
  const { t, lang } = useLanguage();
  const { campaignId } = useParams<{ campaignId: string }>();
  const [campaign, setCampaign] = useState<Campaign | null>(null);
  const [form, setForm] = useState<Form | null>(null);
  const [modules, setModules] = useState<ModuleManifest[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    if (!campaignId) return;
    setLoading(true);
    setError(null);
    try {
      const c = await api.getCampaign(campaignId);
      setCampaign(c);
      rememberCampaign(c.id, c.title);
      if (c.form_id) {
        try {
          setForm(await api.getForm(campaignId));
        } catch {
          setForm(null);
        }
      } else {
        setForm(null);
      }
      const { voting_methods } = await api.listModules();
      setModules(voting_methods);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t("campaignManage.loadError"));
    } finally {
      setLoading(false);
    }
  }, [campaignId]);

  useEffect(() => {
    load();
  }, [load]);

  async function handlePublish() {
    if (!campaignId) return;
    setBusy(true);
    setError(null);
    try {
      setCampaign(await api.publishCampaign(campaignId));
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t("campaignManage.publishError"));
    } finally {
      setBusy(false);
    }
  }

  async function handleClose() {
    if (!campaignId) return;
    setBusy(true);
    setError(null);
    try {
      setCampaign(await api.closeCampaign(campaignId));
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t("campaignManage.closeError"));
    } finally {
      setBusy(false);
    }
  }

  if (loading) return <p className="text-sm text-muted">{t("common.loading")}</p>;
  if (error && !campaign) return <Alert>{error}</Alert>;
  if (!campaign || !campaignId) return null;

  return (
    <div className="flex flex-col gap-8">
      <Link to="/admin/campagnes" className="text-xs text-muted hover:text-ink">
        {t("campaignManage.back")}
      </Link>
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-ink">{campaign.title}</h1>
          <div className="mt-2 flex items-center gap-3">
            <StatusBadge status={campaign.status} />
            <span className="text-xs text-muted">
              {t("campaignManage.createdOn", {
                date: new Date(campaign.created_at).toLocaleDateString(lang),
              })}
            </span>
          </div>
        </div>
        <div className="flex gap-3">
          {campaign.status === "Draft" && form && (
            <Button onClick={handlePublish} disabled={busy}>
              {t("campaignManage.publish")}
            </Button>
          )}
          {campaign.status === "Published" && (
            <Button variant="secondary" onClick={handleClose} disabled={busy}>
              {t("campaignManage.close")}
            </Button>
          )}
        </div>
      </div>

      {error && <Alert>{error}</Alert>}

      {campaign.status === "Draft" && !form && (
        <FormBuilder campaignId={campaignId} onCreated={load} />
      )}

      {campaign.status === "Draft" && form && (
        <Card>
          <p className="text-sm text-ink-soft">
            {t("campaignManage.formReadyPrefix")}
            {form.questions.length}{" "}
            {pluralize(
              form.questions.length,
              t("campaignManage.formReadyQuestion"),
              t("campaignManage.formReadyQuestions"),
            )}
            {t("campaignManage.formReadySuffix")}
          </p>
        </Card>
      )}

      {campaign.status !== "Draft" && form && (
        <div className="flex flex-col gap-6">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold text-ink">{t("campaignManage.questions")}</h2>
            {form.questions.length > 1 && (
              <Link
                to={`/admin/campagnes/${campaignId}/analyse`}
                className="text-sm text-accent hover:underline"
              >
                {t("campaignManage.analysisLink")}
              </Link>
            )}
          </div>
          {form.questions.map((question) => (
            <div key={question.id} className="flex flex-col gap-2">
              {campaign.status === "Published" && (
                <div className="flex flex-wrap gap-4">
                  <Link
                    to={`/campagnes/${campaignId}/questions/${question.id}/voter`}
                    className="self-start text-sm text-accent hover:underline"
                  >
                    {t("campaignManage.openVote")}
                  </Link>
                  <Link
                    to={`/campagnes/${campaignId}/questions/${question.id}/inviter`}
                    className="self-start text-sm text-accent hover:underline"
                  >
                    {t("campaignManage.openInvite")}
                  </Link>
                </div>
              )}
              {question.question_type.type === "single_choice" ||
              question.question_type.type === "multiple_choice" ? (
                <TallyPanel
                  campaignId={campaignId}
                  question={question}
                  modules={modules}
                  canTally={campaign.status === "Published" || campaign.status === "Closed"}
                />
              ) : question.question_type.type === "majority_judgment" ? (
                <MajorityJudgmentPanel
                  campaignId={campaignId}
                  question={question}
                  canTally={campaign.status === "Published" || campaign.status === "Closed"}
                />
              ) : (
                <ResponsesPanel campaignId={campaignId} question={question} />
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
