# Direction visuelle — AgoraVote frontend

## Itération 7 — pivot sur les planches UX fournies

La direction ci-dessous ("papier de scrutin civique") a été choisie en
itération 5 sans jamais avoir vu les six planches d'interface que le
cahier des charges référence pourtant au §10 ("illustration de cadrage
UX fournie par le porteur de projet") — elles n'avaient simplement
jamais été transmises à ce stade du projet. Une fois reçues (itération
7, cf. `docs/DEVLOG.md`), l'écart était trop grand pour une simple
retouche : nav latérale jamais construite (alors que cette section le
prévoyait déjà — "cohérent avec les six planches de maquette fournies
au tout début du projet"), accent bleu et cartes statistiques colorées
au lieu du vert civique unique, densité d'information nettement plus
SaaS que "sobre, civique". Remplacée intégralement plutôt que mélangée
avec l'ancienne direction, pour rester cohérente d'un écran à l'autre.

La section "Plan (avant construction)" ci-dessous garde son contexte
historique (elle documente une vraie décision, prise avec l'information
disponible à ce moment) mais **ne décrit plus l'état actuel** — voir
"Direction actuelle" juste en dessous.

## Direction actuelle (itération 7)

**Couleur** — cf. `src/index.css` pour les valeurs exactes :
- `--color-ink` `#101828`, `--color-paper` `#F9FAFB`,
  `--color-surface` `#FFFFFF`, `--color-line` `#E4E7EC`,
  `--color-muted` `#667085` — fond très clair, quasi blanc, plutôt que
  le gris-vert chaud de l'itération 5.
- `--color-accent` `#2A78D6` — bleu, pas vert : les planches fournies
  l'utilisent pour toute action affirmative (Enregistrer, Suivant,
  Créer un compte) et pour l'état actif de la nav latérale. Toujours
  **un seul accent**, toujours réservé à l'action, pas à la décoration
  — ce principe-là n'a pas changé depuis l'itération 5.
- Palette catégorielle fixe à 8 teintes (`--color-chart-1` à `-8`) et
  palette de statut (`--color-status-good`, `-warning`) — prises de la
  skill `dataviz` disponible dans cet environnement (règle : assigner
  les teintes catégorielles dans un ordre fixe, jamais recyclé ; jamais
  de "rainbow" arbitraire) plutôt qu'improvisées. Revalidées avec son
  script (`validate_palette.js`) avant tout usage — cf. `docs/DEVLOG.md`
  itération 7.

**Type** — une seule famille, Inter (grotesque), pour tout : titres et
corps. La serif éditoriale Fraunces de l'itération 5 est retirée —
aucune des planches fournies n'utilise de serif, et en garder une
séparée du reste aurait recréé l'écart entre plan et réalisation
constaté sur la nav latérale. Toujours auto-hébergée via `@fontsource`
(§1.1 du cahier des charges — pas de dépendance à un service tiers pour
s'afficher).

**Layout** :
- Admin (tableau de bord, campagnes, modules, gestion de campagne,
  analyse) : nav latérale fixe en desktop, tiroir superposé fermé par
  défaut sous 1024px (`AdminLayout.tsx`) — le §11 du cahier des charges
  ("administration desktop-first, utilisable sur tablette") n'excuse
  pas un rendu cassé en dessous de ce seuil ; bug réel trouvé et corrigé
  en itération 7 (nav à largeur fixe écrasant tout le contenu en
  mobile), cf. `docs/DEVLOG.md`.
- Citoyen (vote, résultats publics) : reste sur `Shell.tsx` (nav du
  haut simple), pas la nav latérale — cohérent avec le fait qu'aucune
  des planches montrant l'écran de vote (planche 4) n'a de nav
  latérale non plus.
- Cartes statistiques (tableau de bord) : puce colorée + valeur +
  libellé, une teinte catégorielle fixe par carte (jamais arbitraire).

**Principes qui n'ont pas changé depuis l'itération 5** :
- Un accent, utilisé avec parcimonie — pas de dégradés, pas d'ombres
  décoratives systématiques.
- Aucune fonctionnalité fictive : la nav latérale affiche les six
  entrées des planches, mais "Utilisateurs" et "Paramètres" (aucun
  backend derrière) sont grisées avec un badge "Bientôt" plutôt que
  cliquables vers du vide.
- Aucune donnée fictive : l'écran "Analyse" (planche 6) est
  volontairement plus modeste que la planche fournie — démographie et
  séries temporelles n'existent pas dans le modèle de données actuel,
  jamais simulées pour autant, cf. `docs/DEVLOG.md` itération 7 et
  `docs/ROADMAP.md`.

## Plan (avant construction) — itération 5, contexte historique

**Sujet** : un outil de vote et de consultation citoyenne, libre et
auto-hébergeable. L'interface doit inspirer la confiance et la
lisibilité — pas vendre un produit SaaS.

**Couleur** — 6 valeurs nommées :
- `--color-ink` `#14213D` — texte principal, encre plutôt que noir pur
- `--color-paper` `#F5F6F1` — fond, gris-vert chaud plutôt que crème
- `--color-surface` `#FFFFFF` — cartes/panneaux
- `--color-line` `#DDD9C9` — filets, séparateurs
- `--color-muted` `#5B6472` — texte secondaire
- `--color-accent` `#2F6F4E` — **un seul accent**, réservé aux actions
  affirmatives (voter, publier, confirmer) — un vert civique, pas le
  bleu SaaS générique ni le terracotta par défaut

**Type** — deux familles, rôles distincts :
- Fraunces (serif éditoriale) pour les titres — évoque le document
  officiel/le bulletin de vote, pas un générateur de logo
- Inter (grotesque) pour l'interface et le corps — très lisible aux
  petites tailles
- Auto-hébergées via `@fontsource` (pas de CDN Google Fonts) :
  cohérent avec le principe d'auto-hébergement déjà posé pour le
  backend (§1.1 du cahier des charges) — l'interface ne doit pas
  dépendre d'un service tiers pour s'afficher.

**Layout** :
- Admin : navigation latérale + contenu (cohérent avec les six
  planches de maquette fournies au tout début du projet)
- Citoyen (vote) : colonne centrale unique, une question à la fois,
  indicateur "Question X sur Y" — ici la numérotation EST justifiée
  (c'est une vraie séquence), contrairement à une numérotation
  décorative

**Principes** :
- Un accent, utilisé avec parcimonie — pas de dégradés, pas d'ombres
  décoratives systématiques sur chaque carte
- Aucune majuscule intégrale pour les libellés ; pas d'eyebrow
  générique au-dessus de chaque titre
- Le vote se sent solennel : peu de distraction visuelle sur l'écran
  de vote citoyen, contrairement au tableau de bord admin plus dense

### Revue contre le brief (itération 5)

Le risque générique ici aurait été : fond crème + serif à fort
contraste + accent terracotta (le combo IA par défaut) — évité en
choisissant un vert civique et un fond gris-vert plutôt que crème. Le
risque SaaS générique (cartes identiques à ombre grise, bleu partout)
— évité en réservant l'accent aux seules actions qui font avancer un
processus démocratique (voter, publier), le reste restant sobre.
Rétrospectivement (itération 7) : le "risque SaaS générique" évité ici
est en réalité proche de ce que les planches réelles du porteur de
projet demandaient — la revue avait raisonné dans le vide, faute d'y
avoir eu accès.
