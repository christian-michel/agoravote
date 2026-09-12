# Direction visuelle — AgoraVote frontend

## Plan (avant construction)

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

## Revue contre le brief

Le risque générique ici aurait été : fond crème + serif à fort
contraste + accent terracotta (le combo IA par défaut) — évité en
choisissant un vert civique et un fond gris-vert plutôt que crème. Le
risque SaaS générique (cartes identiques à ombre grise, bleu partout)
— évité en réservant l'accent aux seules actions qui font avancer un
processus démocratique (voter, publier), le reste restant sobre.
