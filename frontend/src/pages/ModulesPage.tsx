import { useEffect, useState } from "react";
import { api, ApiError, type ModuleManifest } from "../api/client";
import { useLanguage } from "../i18n/LanguageContext";
import { Alert, Card } from "../components/ui";
import { getMethodCopy } from "../lib/methodCopy";

/** Écran "Modules" (16 du §10.1 du cahier des charges) — jusqu'ici
 * consommé silencieusement par le sélecteur de méthode de
 * `CampaignManage`, jamais montré comme écran à part entière. */
export default function ModulesPage() {
  const { t } = useLanguage();
  const [modules, setModules] = useState<ModuleManifest[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api
      .listModules()
      .then(({ voting_methods }) => setModules(voting_methods))
      .catch((err) => setError(err instanceof ApiError ? err.message : t("modules.loadError")))
      .finally(() => setLoading(false));
  }, [t]);

  return (
    <div className="flex flex-col gap-8">
      <div>
        <h1 className="text-2xl font-semibold text-ink">{t("modules.title")}</h1>
        <p className="mt-1 text-sm text-muted">{t("modules.subtitle")}</p>
      </div>

      {error && <Alert>{error}</Alert>}
      {loading && <p className="text-sm text-muted">{t("common.loading")}</p>}

      <div className="grid gap-4 sm:grid-cols-2">
        {modules.map((module) => {
          const copy = getMethodCopy(module.id, t);
          return (
            <Card key={module.id}>
              <div className="flex items-center justify-between gap-3">
                <h2 className="font-medium text-ink">{copy?.label ?? module.id}</h2>
                <span className="rounded-full bg-paper px-2 py-0.5 text-xs text-muted">
                  v{module.version}
                </span>
              </div>
              {copy && <p className="mt-2 text-sm text-ink-soft">{copy.description}</p>}
              <dl className="mt-4 flex flex-col gap-1 text-xs text-muted">
                <div className="flex gap-2">
                  <dt className="shrink-0 font-medium">{t("modules.inputs")}</dt>
                  <dd>{module.inputs.join(", ") || "—"}</dd>
                </div>
                <div className="flex gap-2">
                  <dt className="shrink-0 font-medium">{t("modules.parameters")}</dt>
                  <dd>{module.parameters.join(", ") || "—"}</dd>
                </div>
                <div className="flex gap-2">
                  <dt className="shrink-0 font-medium">{t("modules.outputs")}</dt>
                  <dd>{module.outputs.join(", ") || "—"}</dd>
                </div>
              </dl>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
