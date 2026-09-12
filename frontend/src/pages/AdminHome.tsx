import { useEffect, useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { api, ApiError, type Campaign } from "../api/client";
import { Alert, Button, Card, Field, StatCard, StatusBadge } from "../components/ui";
import { CampaignsIcon, PlusIcon } from "../components/icons";
import { listRecentCampaigns, rememberCampaign } from "../lib/recentCampaigns";

/** Compte les campagnes mémorisées par statut, pour les cartes
 * statistiques du tableau de bord (planche 1). Toujours dérivé d'un
 * vrai `GET /campaigns/:id` par campagne mémorisée — jamais un chiffre
 * inventé — cf. `lib/recentCampaigns.ts` pour la limite déjà assumée
 * (liste propre à ce navigateur, pas une vraie liste d'organisation).
 */
function useDashboardStats() {
  const [campaigns, setCampaigns] = useState<Campaign[] | null>(null);

  useEffect(() => {
    const recent = listRecentCampaigns();
    if (recent.length === 0) {
      setCampaigns([]);
      return;
    }
    Promise.all(
      recent.map((c) =>
        api.getCampaign(c.id).catch((err) => {
          if (err instanceof ApiError && err.status === 404) return null;
          throw err;
        }),
      ),
    ).then((results) => setCampaigns(results.filter((c): c is Campaign => c !== null)));
  }, []);

  return campaigns;
}

export default function AdminHome() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const [title, setTitle] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const recent = listRecentCampaigns();
  const campaigns = useDashboardStats();

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

  const drafts = campaigns?.filter((c) => c.status === "Draft").length;
  const published = campaigns?.filter((c) => c.status === "Published").length;
  const closed = campaigns?.filter((c) => c.status === "Closed").length;

  return (
    <div className="flex flex-col gap-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-ink">Tableau de bord</h1>
          <p className="mt-1 text-sm text-muted">Bonjour {user?.display_name}.</p>
        </div>
      </div>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard
          label="Campagnes suivies"
          value={campaigns?.length ?? "…"}
          hue="var(--color-chart-1)"
          icon={<CampaignsIcon width={20} height={20} />}
        />
        <StatCard
          label="Brouillons"
          value={drafts ?? "…"}
          hue="var(--color-chart-2)"
          icon={<CampaignsIcon width={20} height={20} />}
        />
        <StatCard
          label="Publiées"
          value={published ?? "…"}
          hue="var(--color-status-good)"
          icon={<CampaignsIcon width={20} height={20} />}
        />
        <StatCard
          label="Clôturées"
          value={closed ?? "…"}
          hue="var(--color-chart-7)"
          icon={<CampaignsIcon width={20} height={20} />}
        />
      </div>

      <div className="grid gap-8 lg:grid-cols-[1fr_1.3fr]">
        <Card>
          <h2 className="font-medium text-ink">Nouvelle campagne</h2>
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
              <PlusIcon width={16} height={16} />
              {submitting ? "Création…" : "Créer la campagne"}
            </Button>
          </form>
        </Card>

        <Card>
          <div className="flex items-center justify-between">
            <h2 className="font-medium text-ink">Campagnes récentes</h2>
            <Link to="/admin/campagnes" className="text-xs text-accent hover:underline">
              Voir tout →
            </Link>
          </div>
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
              {recent.slice(0, 5).map((c) => {
                const full = campaigns?.find((full) => full.id === c.id);
                return (
                  <li key={c.id}>
                    <Link
                      to={`/admin/campagnes/${c.id}`}
                      className="flex items-center justify-between gap-3 rounded-md border border-line px-3 py-2 text-sm hover:border-accent"
                    >
                      <span className="truncate text-ink">{c.title}</span>
                      <span className="flex shrink-0 items-center gap-2">
                        {full && <StatusBadge status={full.status} />}
                        <span className="text-xs text-muted">
                          {new Date(c.visitedAt).toLocaleDateString("fr-FR")}
                        </span>
                      </span>
                    </Link>
                  </li>
                );
              })}
            </ul>
          )}
        </Card>
      </div>
    </div>
  );
}
