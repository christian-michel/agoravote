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
import { createContext, useContext, useState, type ReactNode } from "react";
import { api, ApiError, getStoredToken, setStoredToken, type User } from "../api/client";

interface AuthState {
  user: User | null;
  /** `undefined` tant qu'on n'a pas fini de vérifier un jeton déjà
   * stocké au chargement de la page — permet à l'UI d'afficher un état
   * de chargement plutôt que de flasher "non connecté" puis "connecté". */
  status: "authenticated" | "anonymous";
  login: (email: string, password: string) => Promise<void>;
  register: (input: { organizationId: string; email: string; password: string; displayName: string }) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthState | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  // L'API n'expose pas de route "qui suis-je" à partir d'un jeton seul
  // (cf. docs/openapi.yaml) : on ne peut donc pas revalider un jeton
  // stocké au rechargement de la page sans connaître déjà
  // l'utilisateur associé. On se contente ici de savoir qu'un jeton
  // existe ; la première requête protégée qui échouerait en 401 (cf.
  // api/client.ts, ApiError) doit alors déclencher une déconnexion
  // propre côté appelant. Calculé en initialiseur paresseux plutôt
  // qu'un effet : c'est une dérivation synchrone d'un état déjà
  // disponible au premier rendu, pas une synchronisation avec un
  // système externe — cf. la règle react/set-state-in-effect.
  const [status, setStatus] = useState<AuthState["status"]>(() =>
    getStoredToken() ? "authenticated" : "anonymous",
  );

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
