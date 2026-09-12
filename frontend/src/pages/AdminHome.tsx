import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { api, ApiError } from "../api/client";
import { Alert, Button, Card, Field } from "../components/ui";
import { listRecentCampaigns, rememberCampaign } from "../lib/recentCampaigns";

export default function AdminHome() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const [title, setTitle] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const recent = listRecentCampaigns();

  async function handleCreate(event: FormEvent) {
    event.preventDefault();
    if (!user) return;
    setError(null);
    setSubmitting(true);
    try {
      const campaign = await api.createCampaign({
        organization_id: user.organization_id,
        title,
      });
      rememberCampaign(campaign.id, campaign.title);
      navigate(`/admin/campagnes/${campaign.id}`);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "La création a échoué.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="flex flex-col gap-10">
      <div>
        <h1 className="font-display text-3xl font-medium text-ink">Tableau de bord</h1>
        <p className="mt-1 text-sm text-muted">Bonjour {user?.display_name}.</p>
      </div>

      <div className="grid gap-8 md:grid-cols-[1fr_1.3fr]">
        <Card>
          <h2 className="font-display text-lg font-medium text-ink">Nouvelle campagne</h2>
          <form onSubmit={handleCreate} className="mt-4 flex flex-col gap-4">
            {error && <Alert>{error}</Alert>}
            <Field
              label="Titre de la campagne"
              name="title"
              placeholder="Budget participatif 2027"
              required
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            />
            <Button type="submit" disabled={submitting}>
              {submitting ? "Création…" : "Créer la campagne"}
            </Button>
          </form>
        </Card>

        <Card>
          <h2 className="font-display text-lg font-medium text-ink">Campagnes récentes</h2>
          <p className="mt-1 text-xs text-muted">
            Liste mémorisée dans ce navigateur — pas encore une vraie liste côté
            serveur (aucune route ne l'expose pour l'instant).
          </p>
          {recent.length === 0 ? (
            <p className="mt-4 text-sm text-muted">
              Aucune campagne visitée depuis ce navigateur pour l'instant.
            </p>
          ) : (
            <ul className="mt-4 flex flex-col gap-2">
              {recent.map((c) => (
                <li key={c.id}>
                  <Link
                    to={`/admin/campagnes/${c.id}`}
                    className="flex items-center justify-between rounded-md border border-line px-3 py-2 text-sm hover:border-accent"
                  >
                    <span className="text-ink">{c.title}</span>
                    <span className="text-xs text-muted">
                      {new Date(c.visitedAt).toLocaleDateString("fr-FR")}
                    </span>
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>
    </div>
  );
}
