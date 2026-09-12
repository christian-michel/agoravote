import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { Button } from "./ui";

/** En-tête commun à tout le site. Volontairement discret : le
 * contenu de chaque page porte l'attention, pas le chrome autour —
 * cf. DESIGN.md, "un accent, utilisé avec parcimonie". */
export function Shell({ children }: { children: ReactNode }) {
  const { user, status, logout } = useAuth();

  return (
    <div className="min-h-screen bg-paper text-ink">
      <header className="border-b border-line bg-surface">
        <div className="mx-auto flex max-w-5xl items-center justify-between px-6 py-4">
          <Link to="/" className="font-display text-lg font-medium text-ink">
            AgoraVote
          </Link>
          <nav className="flex items-center gap-4 text-sm">
            {status === "authenticated" && user ? (
              <>
                <Link to="/admin" className="text-ink-soft hover:text-ink">
                  Tableau de bord
                </Link>
                <span className="text-muted">{user.display_name}</span>
                <Button variant="secondary" onClick={() => logout()}>
                  Se déconnecter
                </Button>
              </>
            ) : status === "anonymous" ? (
              <>
                <Link to="/connexion" className="text-ink-soft hover:text-ink">
                  Se connecter
                </Link>
                <Link to="/inscription">
                  <Button>Créer un compte</Button>
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
