import { useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { api, ApiError, type Campaign, type Form, type ModuleManifest } from "../api/client";
import { Alert, Button, Card, StatusBadge } from "../components/ui";
import { FormBuilder } from "../components/FormBuilder";
import { TallyPanel } from "../components/TallyPanel";
import { rememberCampaign } from "../lib/recentCampaigns";

export default function CampaignManage() {
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
      setError(err instanceof ApiError ? err.message : "Impossible de charger la campagne.");
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
      setError(err instanceof ApiError ? err.message : "La publication a échoué.");
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
      setError(err instanceof ApiError ? err.message : "La clôture a échoué.");
    } finally {
      setBusy(false);
    }
  }

  if (loading) return <p className="text-sm text-muted">Chargement…</p>;
  if (error && !campaign) return <Alert>{error}</Alert>;
  if (!campaign || !campaignId) return null;

  return (
    <div className="flex flex-col gap-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-display text-3xl font-medium text-ink">{campaign.title}</h1>
          <div className="mt-2 flex items-center gap-3">
            <StatusBadge status={campaign.status} />
            <span className="text-xs text-muted">
              Créée le {new Date(campaign.created_at).toLocaleDateString("fr-FR")}
            </span>
          </div>
        </div>
        <div className="flex gap-3">
          {campaign.status === "Draft" && form && (
            <Button onClick={handlePublish} disabled={busy}>
              Publier la campagne
            </Button>
          )}
          {campaign.status === "Published" && (
            <Button variant="secondary" onClick={handleClose} disabled={busy}>
              Clôturer la campagne
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
            Le formulaire est prêt ({form.questions.length} question
            {form.questions.length > 1 ? "s" : ""}). Publiez la campagne pour
            commencer à recevoir des votes.
          </p>
        </Card>
      )}

      {campaign.status !== "Draft" && form && (
        <div className="flex flex-col gap-6">
          <h2 className="font-display text-xl font-medium text-ink">Questions</h2>
          {form.questions.map((question) => (
            <div key={question.id} className="flex flex-col gap-2">
              {campaign.status === "Published" && (
                <Link
                  to={`/campagnes/${campaignId}/questions/${question.id}/voter`}
                  className="self-start text-sm text-accent hover:underline"
                >
                  Ouvrir l'écran de vote citoyen →
                </Link>
              )}
              <TallyPanel
                campaignId={campaignId}
                question={question}
                modules={modules}
                canTally={campaign.status === "Published" || campaign.status === "Closed"}
              />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
