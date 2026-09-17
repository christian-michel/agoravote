import { Link } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { useLanguage } from "../i18n/LanguageContext";
import { Button, Card } from "../components/ui";

export default function Home() {
  const { status } = useAuth();
  const { t } = useLanguage();

  return (
    <div className="flex flex-col gap-10">
      <div className="max-w-2xl">
        <h1 className="font-display text-4xl font-medium leading-tight text-ink">
          {t("home.title")}
        </h1>
        <p className="mt-4 text-base text-ink-soft">{t("home.subtitle")}</p>
      </div>

      <div className="grid gap-6 sm:grid-cols-2">
        <Card>
          <h2 className="font-display text-xl font-medium text-ink">{t("home.organizeTitle")}</h2>
          <p className="mt-2 text-sm text-ink-soft">{t("home.organizeBody")}</p>
          <div className="mt-4">
            {status === "authenticated" ? (
              <Link to="/admin">
                <Button>{t("home.goDashboard")}</Button>
              </Link>
            ) : status === "anonymous" ? (
              <Link to="/inscription">
                <Button>{t("home.createOrganizer")}</Button>
              </Link>
            ) : null}
          </div>
        </Card>

        <Card>
          <h2 className="font-display text-xl font-medium text-ink">{t("home.participateTitle")}</h2>
          <p className="mt-2 text-sm text-ink-soft">{t("home.participateBody")}</p>
        </Card>
      </div>
    </div>
  );
}
