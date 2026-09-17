import { useState, type ReactNode } from "react";
import { Link, useLocation } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { useLanguage } from "../i18n/LanguageContext";
import { LanguageSwitcher } from "../i18n/LanguageSwitcher";
import type { TranslationKey } from "../i18n/translations/fr";
import {
  CampaignsIcon,
  DashboardIcon,
  ModulesIcon,
  SettingsIcon,
  UsersIcon,
} from "./icons";

interface NavItem {
  labelKey: TranslationKey;
  href?: string;
  icon: (props: { className?: string }) => ReactNode;
}

/** `href` absent = pas encore construit (pas de route/endpoint réel
 * derrière) — affiché grisé plutôt qu'omis, pour rester fidèle à la
 * disposition des planches fournies (cf. docs/DEVLOG.md itération 7)
 * sans jamais faire croire qu'une fonction existe. */
const NAV_ITEMS: NavItem[] = [
  { labelKey: "nav.dashboard", href: "/admin", icon: DashboardIcon },
  { labelKey: "nav.campaigns", href: "/admin/campagnes", icon: CampaignsIcon },
  { labelKey: "nav.modules", href: "/admin/modules", icon: ModulesIcon },
  { labelKey: "nav.users", icon: UsersIcon },
  { labelKey: "nav.settings", icon: SettingsIcon },
];

/** Disposition avec nav latérale pour les écrans d'administration
 * (planches 1-3, 5-6 du cahier des charges §10) — distincte de
 * `Shell.tsx` (nav du haut), gardée pour les écrans publics/citoyens
 * (accueil, connexion, vote, résultats), cf. §11 "citoyen mobile-first,
 * administration desktop-first". "Desktop-first" n'excuse pas un rendu
 * cassé en dessous : la nav latérale devient un tiroir superposé
 * (fermé par défaut) sous le seuil `lg`, plutôt qu'une largeur fixe qui
 * écraserait le contenu (bug constaté à la vérification navigateur,
 * cf. docs/DEVLOG.md itération 7). */
export function AdminLayout({ children }: { children: ReactNode }) {
  const { user, logout } = useAuth();
  const { t } = useLanguage();
  const location = useLocation();
  const [drawerOpen, setDrawerOpen] = useState(false);

  const sidebarContent = (
    <>
      <div className="flex items-center gap-2 border-b border-line px-6 py-5">
        <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-accent text-sm font-bold text-white">
          A
        </span>
        <Link to="/" className="text-base font-semibold text-ink">
          AgoraVote
        </Link>
      </div>

      <nav className="flex flex-1 flex-col gap-1 px-3 py-4">
        {NAV_ITEMS.map((item) => {
          // "/admin" exige une égalité stricte (sinon il resterait actif
          // sur "/admin/campagnes", qui commence aussi par "/admin") ;
          // les autres entrées restent actives sur leurs sous-routes
          // (ex. "/admin/campagnes/:id/analyse" garde "Campagnes" actif).
          const active =
            item.href != null &&
            location.pathname.startsWith(item.href) &&
            (item.href === "/admin" ? location.pathname === "/admin" : true);
          const Icon = item.icon;
          if (!item.href) {
            return (
              <span
                key={item.labelKey}
                className="flex cursor-not-allowed items-center justify-between gap-3 rounded-lg px-3 py-2 text-sm text-muted/70"
                title={t("nav.soonTitle")}
              >
                <span className="flex items-center gap-3">
                  <Icon className="shrink-0" />
                  {t(item.labelKey)}
                </span>
                <span className="rounded-full bg-paper px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-muted">
                  {t("nav.soon")}
                </span>
              </span>
            );
          }
          return (
            <Link
              key={item.labelKey}
              to={item.href}
              onClick={() => setDrawerOpen(false)}
              className={`flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors ${
                active ? "bg-accent-soft text-accent-strong" : "text-ink-soft hover:bg-paper hover:text-ink"
              }`}
            >
              <Icon className="shrink-0" />
              {t(item.labelKey)}
            </Link>
          );
        })}
      </nav>

      <div className="border-t border-line px-4 py-3">
        <LanguageSwitcher className="w-full" />
      </div>

      {user && (
        <div className="flex items-center gap-3 border-t border-line px-4 py-4">
          <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-accent-soft text-xs font-semibold text-accent-strong">
            {user.display_name.slice(0, 1).toUpperCase()}
          </span>
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-medium text-ink">{user.display_name}</p>
            <button type="button" onClick={() => logout()} className="text-xs text-muted hover:text-ink">
              {t("nav.logout")}
            </button>
          </div>
        </div>
      )}
    </>
  );

  return (
    <div className="flex min-h-screen bg-paper text-ink">
      {/* Barre mobile (< lg) : logo + bouton pour ouvrir le tiroir de nav. */}
      <div className="fixed inset-x-0 top-0 z-30 flex items-center gap-3 border-b border-line bg-surface px-4 py-3 lg:hidden">
        <button
          type="button"
          onClick={() => setDrawerOpen(true)}
          className="flex h-9 w-9 items-center justify-center rounded-md border border-line text-ink-soft"
          aria-label={t("nav.openMenu")}
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round">
            <path d="M4 6h16M4 12h16M4 18h16" />
          </svg>
        </button>
        <Link to="/" className="text-base font-semibold text-ink">
          AgoraVote
        </Link>
      </div>

      {drawerOpen && (
        <button
          type="button"
          aria-label={t("nav.closeMenu")}
          onClick={() => setDrawerOpen(false)}
          className="fixed inset-0 z-30 bg-ink/30 lg:hidden"
        />
      )}

      <aside
        className={`fixed inset-y-0 left-0 z-40 flex w-64 shrink-0 flex-col border-r border-line bg-surface transition-transform duration-200 lg:static lg:z-auto lg:translate-x-0 ${
          drawerOpen ? "translate-x-0" : "-translate-x-full"
        }`}
      >
        {sidebarContent}
      </aside>

      <main className="min-w-0 flex-1 px-4 pb-8 pt-20 sm:px-8 sm:pt-8 lg:pt-8">
        <div className="mx-auto max-w-6xl">{children}</div>
      </main>
    </div>
  );
}
