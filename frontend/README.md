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
| Tableau de bord admin | `/admin` | Fait — cartes statistiques réelles (dérivées des campagnes mémorisées, cf. `src/lib/recentCampaigns.ts`), création de campagne |
| Campagnes | `/admin/campagnes` | Fait (itération 7) — reprend la même limite que le tableau de bord : liste propre à ce navigateur, pas une vraie liste serveur |
| Modules | `/admin/modules` | Fait (itération 7) — `GET /modules`, déjà réel, jamais montré comme écran à part entière auparavant |
| Gestion de campagne | `/admin/campagnes/:id` | Fait — bibliothèque de questions (les six types acceptés par l'API depuis l'itération 7), sélecteur de méthode en cartes, publication, clôture, dépouillement |
| Analyse et visualisation | `/admin/campagnes/:id/analyse` | Fait (itération 7), scope volontairement réduit — comparaison des résultats déjà calculés entre questions ; pas de démographie ni de série temporelle (données inexistantes, cf. `docs/ROADMAP.md`) |
| Vote citoyen | `/campagnes/:id/questions/:qid/voter` | Fait |
| Résultats publics | `/campagnes/:id/questions/:qid/resultats` | Fait — jauge de participation, donut, barres, méthodologie dépliable |

## Ce qui n'est PAS fait

- **Vérification visuelle** : faite à deux reprises, cf. `docs/DEVLOG.md`
  — itération 6 (premier passage, a révélé et corrigé le bug `/auth/me`
  et un conflit de peer-dependency `typescript`) et itération 7 (refonte
  sur les planches UX fournies par le porteur de projet, a révélé et
  corrigé deux bugs de responsive mobile : nav du haut chevauchée sous
  400px pour un utilisateur connecté, et nav latérale à largeur fixe
  rendant tout le contenu admin inutilisable sous 1024px).
- Liste des campagnes/modules côté serveur (cf. tableau ci-dessus —
  `GET /campaigns` n'existe toujours pas côté API).
- Écran de permissions fines / journal d'audit (dépend des mêmes
  limites déjà documentées côté backend, cf. `README.md` racine) —
  affiché grisé avec un badge "Bientôt" dans la nav latérale plutôt
  qu'omis, sans jamais être cliquable.
- Démographie et séries temporelles dans l'écran Analyse (cf.
  `docs/ROADMAP.md` — nécessite une décision produit sur la collecte
  de ces données, pas seulement un ajout technique).
- Internationalisation de l'interface elle-même (le contenu des
  campagnes est déjà multilingue côté modèle — `prompt`/`labels` sont
  des dictionnaires par langue — mais l'interface n'affiche que `fr`
  pour l'instant).
