/**
 * Contexte d'authentification — enveloppe l'état de session
 * (utilisateur courant, jeton) autour de toute l'application.
 *
 * Le jeton est stocké dans `localStorage` (cf. `api/client.ts`) :
 * c'est le choix le plus simple pour une SPA qui reçoit son jeton
 * dans le corps de la réponse JSON (pas un cookie posé par le
 * serveur — l'API ne fait pas ça, cf. `docs/openapi.yaml`). Compromis
 * assumé : `localStorage` est lisible par tout script s'exécutant sur
 * la page (risque XSS) ; une évolution future pourrait passer par un
 * cookie `HttpOnly` posé par un futur endpoint dédié, qui éliminerait
 * ce risque — non fait ici pour rester au plus près de l'API actuelle
 * telle qu'elle existe réellement (cf. docs/SECURITY.md du backend
 * pour la même discipline : documenter les compromis plutôt que les
 * cacher).
 */
import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { api, ApiError, getStoredToken, setStoredToken, type User } from "../api/client";

interface AuthState {
  user: User | null;
  /** `"checking"` tant qu'un jeton stocké n'a pas encore été confirmé
   * contre `GET /auth/me` (juste après un rechargement de page) —
   * permet à l'UI d'afficher un état de chargement plutôt que de
   * flasher "non connecté" puis "connecté", et surtout d'éviter que
   * `status === "authenticated"` coexiste avec `user === null` (bug
   * découvert en vérifiant le frontend dans un vrai navigateur, cf.
   * docs/DEVLOG.md itération 6 : l'en-tête n'affichait alors plus
   * aucun lien, et "Créer la campagne" échouait silencieusement). */
  status: "checking" | "authenticated" | "anonymous";
  login: (email: string, password: string) => Promise<void>;
  register: (input: { organizationId: string; email: string; password: string; displayName: string }) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthState | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  // Calculé en initialiseur paresseux plutôt qu'un effet : c'est une
  // dérivation synchrone d'un état déjà disponible au premier rendu,
  // pas une synchronisation avec un système externe — cf. la règle
  // react/set-state-in-effect. L'effet ci-dessous ne fait que la
  // confirmation réseau qui doit forcément suivre.
  const [status, setStatus] = useState<AuthState["status"]>(() =>
    getStoredToken() ? "checking" : "anonymous",
  );

  // Un jeton en `localStorage` ne suffit pas à connaître
  // l'utilisateur qu'il désigne (il a pu être émis lors d'une session
  // précédente, avant ce rechargement) : `GET /auth/me` le confirme
  // (et remplit `user`), ou révèle qu'il est expiré/invalide, auquel
  // cas on efface la session locale plutôt que de rester bloqué en
  // "authenticated" sans utilisateur.
  useEffect(() => {
    if (status !== "checking") return;
    let cancelled = false;
    api
      .me()
      .then((fetchedUser) => {
        if (cancelled) return;
        setUser(fetchedUser);
        setStatus("authenticated");
      })
      .catch(() => {
        if (cancelled) return;
        setStoredToken(null);
        setStatus("anonymous");
      });
    return () => {
      cancelled = true;
    };
  }, [status]);

  const login: AuthState["login"] = async (email, password) => {
    const result = await api.login({ email, password });
    setStoredToken(result.token);
    setUser(result.user);
    setStatus("authenticated");
  };

  const register: AuthState["register"] = async ({ organizationId, email, password, displayName }) => {
    const result = await api.register({
      organization_id: organizationId,
      email,
      password,
      display_name: displayName,
    });
    setStoredToken(result.token);
    setUser(result.user);
    setStatus("authenticated");
  };

  const logout: AuthState["logout"] = async () => {
    try {
      await api.logout();
    } catch {
      // Une déconnexion échouée côté serveur (jeton déjà expiré, par
      // exemple) ne doit pas empêcher la déconnexion côté client —
      // l'utilisateur veut se déconnecter, on efface l'état local dans
      // tous les cas.
    }
    setStoredToken(null);
    setUser(null);
    setStatus("anonymous");
  };

  return (
    <AuthContext.Provider value={{ user, status, login, register, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthState {
  const ctx = useContext(AuthContext);
  if (!ctx) {
    throw new Error("useAuth doit être utilisé à l'intérieur de <AuthProvider>");
  }
  return ctx;
}

/** Vrai si l'erreur signale un jeton invalide/expiré — utile pour
 * réagir uniformément (rediriger vers /connexion) partout où l'API
 * est appelée, cf. son usage dans les pages admin. */
export function isAuthError(error: unknown): boolean {
  return error instanceof ApiError && error.status === 401;
}
