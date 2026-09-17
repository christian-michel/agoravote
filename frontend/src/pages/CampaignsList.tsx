import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, ApiError, type Campaign } from "../api/client";
import { listRecentCampaigns } from "../lib/recentCampaigns";
import { useLanguage } from "../i18n/LanguageContext";
import { Alert, Button, Card, StatusBadge } from "../components/ui";
import { PlusIcon } from "../components/icons";

/** Écran "Campagnes" (planche 1, entrée de nav) — cf. la limite déjà
 * assumée dans `lib/recentCampaigns.ts` : l'API n'expose aucune route
 * de liste, donc cet écran affiche les campagnes que CE navigateur a
 * déjà créées ou visitées, pas toutes celles de l'organisation. */
export default function CampaignsList() {
  const { t, lang } = useLanguage();
  const [campaigns, setCampaigns] = useState<(Campaign | null)[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const recent = listRecentCampaigns();
    if (recent.length === 0) {
      setLoading(false);
      return;
    }
    Promise.all(
      recent.map((c) =>
        api.getCampaign(c.id).catch((err) => {
          if (err instanceof ApiError && err.status === 404) return null;
          throw err;
        }),
      ),
    )
      .then((results) => setCampaigns(results))
      .catch((err) => setError(err instanceof ApiError ? err.message : t("campaignsList.loadError")))
      .finally(() => setLoading(false));
  }, []);

  const found = campaigns.filter((c): c is Campaign => c !== null);

  return (
    <div className="flex flex-col gap-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-ink">{t("campaignsList.title")}</h1>
          <p className="mt-1 text-sm text-muted">{t("campaignsList.subtitle")}</p>
        </div>
        <Link to="/admin">
          <Button>
            <PlusIcon width={16} height={16} /> {t("campaignsList.new")}
          </Button>
        </Link>
      </div>

      {error && <Alert>{error}</Alert>}
      {loading && <p className="text-sm text-muted">{t("common.loading")}</p>}

      {!loading && found.length === 0 && (
        <Card>
          <p className="text-sm text-muted">{t("campaignsList.none")}</p>
        </Card>
      )}

      <ul className="flex flex-col gap-3">
        {found.map((campaign) => (
          <li key={campaign.id}>
            <Link
              to={`/admin/campagnes/${campaign.id}`}
              className="flex items-center justify-between gap-4 rounded-lg border border-line bg-surface px-5 py-4 transition-colors hover:border-accent"
            >
              <div className="min-w-0">
                <p className="truncate font-medium text-ink">{campaign.title}</p>
                <p className="mt-1 text-xs text-muted">
                  {t("campaignsList.createdOn", {
                    date: new Date(campaign.created_at).toLocaleDateString(lang),
                  })}
                </p>
              </div>
              <StatusBadge status={campaign.status} />
            </Link>
          </li>
        ))}
      </ul>
    </div>
  );
}
