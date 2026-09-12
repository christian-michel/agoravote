# AgoraVote — frontend

Interface web React consommant l'API `agoravote-api` (cf.
`docs/openapi.yaml` à la racine du dépôt, source de vérité du contrat
entre les deux). Voir `DESIGN.md` pour la direction visuelle et sa
justification.

## Démarrer

```bash
npm install
npm run dev
```

Le serveur de développement Vite proxifie `/api/*` vers
`http://localhost:3000` (cf. `vite.config.ts`) — démarrez l'API Rust
en parallèle (`cargo run -p agoravote-api` ou `docker compose up`
depuis la racine) avant de tester le frontend.

## Garder les types synchronisés avec l'API

```bash
npm run gen:api
```

Régénère les types de `src/api/schema.ts` à partir de
`docs/openapi.yaml`. **À relancer après toute modification de l'API**
côté backend (nouvelle route, nouveau champ...) — sinon le frontend
compilera contre un contrat obsolète sans avertissement.

## État — ce qui existe

| Écran | Route | Statut |
|---|---|---|
| Accueil | `/` | Fait |
| Connexion / Inscription | `/connexion`, `/inscription` | Fait |
| Tableau de bord admin | `/admin` | Fait — création de campagne ; la liste "campagnes récentes" est mémorisée dans le navigateur (cf. `src/lib/recentCampaigns.ts`), pas une vraie liste serveur (l'API n'expose pas encore cette route) |
| Gestion de campagne | `/admin/campagnes/:id` | Fait — construction du formulaire (choix unique/multiple uniquement), publication, clôture, dépouillement |
| Vote citoyen | `/campagnes/:id/questions/:qid/voter` | Fait |
| Résultats publics | `/campagnes/:id/questions/:qid/resultats` | Fait |

## Ce qui n'est PAS fait

- **Vérification visuelle** : ce frontend a été construit sans accès à
  un navigateur réel dans l'environnement de développement — la
  compilation TypeScript (`npx tsc -b`), le lint (`npx oxlint`) et le
  build de production (`npm run build`) passent tous sans erreur,
  mais **aucune capture d'écran n'a pu être prise**. Premier test
  visuel à faire : `npm run dev` puis ouvrir `http://localhost:5173`.
- Types de question au-delà de choix unique/multiple (texte, nombre,
  échelle, classement) dans le constructeur de formulaire — l'API les
  supporte déjà (cf. `docs/openapi.yaml`), pas encore l'UI.
- Liste des campagnes côté serveur (cf. tableau ci-dessus).
- Écran de permissions fines / journal d'audit (dépend des mêmes
  limites déjà documentées côté backend, cf. `README.md` racine).
- Internationalisation de l'interface elle-même (le contenu des
  campagnes est déjà multilingue côté modèle — `prompt`/`labels` sont
  des dictionnaires par langue — mais l'interface n'affiche que `fr`
  pour l'instant).
