/**
 * L'API n'expose aucune route "lister les campagnes d'une
 * organisation" (cf. docs/openapi.yaml — seulement GET par id précis).
 * En attendant cette route côté backend (§21, travaux restants), on
 * mémorise localement les campagnes que CET utilisateur a créées ou
 * consultées, pour que le tableau de bord ne soit pas un simple
 * formulaire de création sans aucun rappel de ce qui existe déjà.
 *
 * Limite assumée et documentée : cette liste est propre au navigateur
 * (pas partagée entre appareils), et n'affiche que ce que CET
 * utilisateur a vu — pas toutes les campagnes de l'organisation. Une
 * vraie route `GET /campaigns?organization_id=...` réglerait ça
 * proprement ; ce mécanisme est un pis-aller assumé, pas la solution
 * cible.
 */
const STORAGE_KEY = "agoravote.recent_campaigns";
const MAX_ENTRIES = 20;

export interface RecentCampaign {
  id: string;
  title: string;
  visitedAt: string;
}

export function rememberCampaign(id: string, title: string): void {
  const existing = listRecentCampaigns().filter((c) => c.id !== id);
  const updated = [{ id, title, visitedAt: new Date().toISOString() }, ...existing].slice(
    0,
    MAX_ENTRIES,
  );
  localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
}

export function listRecentCampaigns(): RecentCampaign[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as RecentCampaign[]) : [];
  } catch {
    return [];
  }
}
