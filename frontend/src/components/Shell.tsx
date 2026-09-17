import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { useLanguage } from "../i18n/LanguageContext";
import { LanguageSwitcher } from "../i18n/LanguageSwitcher";
import { Button } from "./ui";

/** En-tête commun à tout le site. Volontairement discret : le
 * contenu de chaque page porte l'attention, pas le chrome autour —
 * cf. DESIGN.md, "un accent, utilisé avec parcimonie". */
export function Shell({ children }: { children: ReactNode }) {
  const { user, status, logout } = useAuth();
  const { t } = useLanguage();

  return (
    <div className="min-h-screen bg-paper text-ink">
      <header className="border-b border-line bg-surface">
        <div className="mx-auto flex max-w-5xl items-center justify-between gap-3 px-4 py-4 sm:px-6">
          <Link to="/" className="shrink-0 font-display text-lg font-medium text-ink">
            AgoraVote
          </Link>
          <nav className="flex items-center gap-3 text-sm sm:gap-4">
            <LanguageSwitcher />
            {status === "authenticated" && user ? (
              <>
                <Link to="/admin" className="hidden text-ink-soft hover:text-ink sm:inline">
                  {t("nav.dashboard")}
                </Link>
                <span className="hidden text-muted sm:inline">{user.display_name}</span>
                <Button variant="secondary" onClick={() => logout()}>
                  {t("nav.logout")}
                </Button>
              </>
            ) : status === "anonymous" ? (
              <>
                <Link to="/connexion" className="text-ink-soft hover:text-ink">
                  {t("shell.login")}
                </Link>
                <Link to="/inscription">
                  <Button>{t("shell.createAccount")}</Button>
                </Link>
              </>
            ) : null}
          </nav>
        </div>
      </header>
      <main className="mx-auto max-w-5xl px-6 py-10">{children}</main>
    </div>
  );
}
