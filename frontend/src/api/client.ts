/**
 * Client API AgoraVote — enveloppe `fetch` typée à partir de
 * `schema.ts` (généré automatiquement depuis `docs/openapi.yaml` via
 * `npm run gen:api`, cf. package.json). Aucun type n'est dupliqué à la
 * main ici : si l'API change, régénérer le schéma fait apparaître les
 * erreurs de compilation aux bons endroits plutôt que des bugs
 * silencieux à l'exécution — même philosophie que le reste du projet
 * (cf. docs/LOGGING.md côté backend, "erreurs silencieuses").
 */
import type { components } from "./schema";

export type Campaign = components["schemas"]["Campaign"];
export type Form = components["schemas"]["Form"];
export type Question = components["schemas"]["Question"];
export type Ballot = components["schemas"]["Ballot"];
export type ResultSet = components["schemas"]["ResultSet"];
export type User = components["schemas"]["User"];
export type ModuleManifest = components["schemas"]["ModuleManifest"];
export type CreateQuestionRequest = components["schemas"]["CreateQuestionRequest"];

/**
 * En développement, Vite proxifie `/api/*` vers `http://localhost:3000`
 * (cf. vite.config.ts) pour éviter les soucis de CORS. En production,
 * le frontend et l'API sont servis derrière le même reverse proxy
 * (cf. docker-compose.yml) sous le même préfixe — donc `/api` marche
 * dans les deux cas sans configuration supplémentaire.
 */
const BASE_URL = "/api";

const TOKEN_STORAGE_KEY = "agoravote.token";

export function getStoredToken(): string | null {
  return localStorage.getItem(TOKEN_STORAGE_KEY);
}

export function setStoredToken(token: string | null): void {
  if (token) {
    localStorage.setItem(TOKEN_STORAGE_KEY, token);
  } else {
    localStorage.removeItem(TOKEN_STORAGE_KEY);
  }
}

/** Erreur applicative : toujours porteuse du message renvoyé par
 * l'API (`{ "error": "..." }`, cf. `ErrorResponse` dans le schéma) —
 * jamais un message générique fabriqué côté client qui masquerait la
 * vraie raison du refus (400 de validation métier, 401, 403...). */
export class ApiError extends Error {
  status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

interface RequestOptions {
  method?: "GET" | "POST";
  body?: unknown;
  /** Force une requête anonyme même si un jeton est stocké — utilisé
   * par le vote anonyme volontaire (cf. §13 : voter avec ou sans
   * identité est un choix, pas une conséquence accidentelle d'avoir
   * un onglet de session ouvert par ailleurs). */
  anonymous?: boolean;
}

async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const headers: Record<string, string> = {};
  if (options.body !== undefined) {
    headers["Content-Type"] = "application/json";
  }
  const token = options.anonymous ? null : getStoredToken();
  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  const response = await fetch(`${BASE_URL}${path}`, {
    method: options.method ?? "GET",
    headers,
    body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
  });

  if (response.status === 204) {
    return undefined as T;
  }

  const data = await response.json().catch(() => null);

  if (!response.ok) {
    const message =
      data && typeof data === "object" && "error" in data
        ? String((data as { error: unknown }).error)
        : `Erreur ${response.status}`;
    throw new ApiError(response.status, message);
  }

  return data as T;
}

export const api = {
  health: () => request<string>("/health"),

  listModules: () =>
    request<{ voting_methods: ModuleManifest[] }>("/modules"),

  register: (body: { organization_id: string; email: string; password: string; display_name: string }) =>
    request<{ token: string; user: User }>("/auth/register", { method: "POST", body }),

  login: (body: { email: string; password: string }) =>
    request<{ token: string; user: User }>("/auth/login", { method: "POST", body }),

  logout: () => request<void>("/auth/logout", { method: "POST" }),

  createCampaign: (body: { organization_id: string; title: string }) =>
    request<Campaign>("/campaigns", { method: "POST", body }),

  getCampaign: (campaignId: string) =>
    request<Campaign>(`/campaigns/${campaignId}`),

  getForm: (campaignId: string) =>
    request<Form>(`/campaigns/${campaignId}/form`),

  createForm: (campaignId: string, body: { questions: CreateQuestionRequest[] }) =>
    request<Form>(`/campaigns/${campaignId}/form`, { method: "POST", body }),

  publishCampaign: (campaignId: string) =>
    request<Campaign>(`/campaigns/${campaignId}/publish`, { method: "POST" }),

  closeCampaign: (campaignId: string) =>
    request<Campaign>(`/campaigns/${campaignId}/close`, { method: "POST" }),

  castBallot: (
    campaignId: string,
    questionId: string,
    body: { selections?: string[]; scores?: Record<string, number> },
    anonymous = false,
  ) =>
    request<Ballot>(`/campaigns/${campaignId}/questions/${questionId}/ballots`, {
      method: "POST",
      body,
      anonymous,
    }),

  tally: (
    campaignId: string,
    questionId: string,
    body: { voting_method_id: string; eligible_voters?: number; quorum?: number; seats?: number },
  ) =>
    request<ResultSet>(`/campaigns/${campaignId}/questions/${questionId}/tally`, {
      method: "POST",
      body,
    }),

  getResults: (campaignId: string, questionId: string) =>
    request<ResultSet>(`/campaigns/${campaignId}/questions/${questionId}/results`),
};
