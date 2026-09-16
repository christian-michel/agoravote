# Feuille de route

Priorités actuelles, consolidées depuis les "prochaines itérations
candidates" de fin de chaque entrée de `docs/DEVLOG.md`. Ce fichier est
la référence unique et à jour — le DEVLOG garde l'historique des
décisions passées, celui-ci ne garde que ce qui reste à faire.

**Convention** : cocher un item ici quand il est fait, avec un renvoi
vers l'entrée DEVLOG correspondante. Ne jamais supprimer un item
coché — l'historique de ce qui a été priorisé et quand a de la valeur.

## Priorité immédiate

- [x] **Vérifier le frontend dans un vrai navigateur.** Fait — cf.
      `docs/DEVLOG.md` itération 6 : parcours complet piloté par
      Chromium/Playwright (desktop + mobile), a révélé et corrigé un
      bug réel (session affichée comme perdue après un rechargement de
      page, `GET /auth/me` ajouté pour le corriger) et un conflit de
      peer-dependency npm (`typescript` ramené à `^5.9.3`).
- [x] **Compiler `agoravote-g1`** avec une toolchain à jour, ajouter un
      test avec un vecteur cryptographique connu. Fait — cf.
      `docs/DEVLOG.md` itération 6 (rustc 1.94, 9 tests verts). Vecteur
      corrigé sr25519 -> **ed25519** en itération 9 (cf. ci-dessous et
      `docs/SECURITY.md` §8) : le schéma d'origine était une hypothèse
      non vérifiée, probablement fausse pour les comptes Ğ1v2 réels.
- [ ] **Confirmer les noms de stockage de `agoravote-g1` (`chain.rs`)
      contre un nœud `gdev` réel.** Toujours bloqué : l'accès réseau à
      l'infrastructure Duniter/Ğ1 reste refusé par la politique réseau
      de tous les environnements de développement utilisés jusqu'ici
      (testé et confirmé à nouveau en itération 8, pas juste supposé
      inchangé) — nécessite un environnement qui a cet accès. Bloquant
      aussi pour reconfirmer le schéma de signature ed25519 (itération
      9) contre une source primaire, faute d'accès à
      `forum.duniter.org`/`git.duniter.org` depuis ici.

## Backend

- [ ] Migration `sqlx` 0.6 → 0.7/0.8 dans `agoravote-store` — la
      contrainte de toolchain qui l'empêchait est levée (rustc 1.94,
      cf. `docs/DEVLOG.md` itération 6), mais c'est un changement d'API
      majeur (macros/offline/executor) qui touche tout le crate déjà
      testé contre PostgreSQL réel ; à faire comme item dédié, pas à la
      volée.

- [ ] `GET /campaigns` (liste, filtrée par organisation) — remplace le
      palliatif de mémorisation locale (`frontend/src/lib/recentCampaigns.ts`),
      utilisé par le tableau de bord ET par l'écran "Campagnes" ajouté
      en itération 7.
- [ ] Permissions fines : une campagne appartient à un organisateur
      précis (`owner_id`), pas à "tout Organizer de l'organisation".
- [ ] Flux d'invitation d'organisateurs — aujourd'hui, `/auth/register`
      attribue directement le rôle `Organizer` à quiconque s'inscrit.
- [x] Validation bulletin ↔ type de question à la soumission. Fait —
      cf. `docs/DEVLOG.md` itération 12 : `cast_ballot` valide
      désormais le bulletin contre le type réel de la question
      (bornes min/max, longueur max, permutation complète pour un
      classement) avant construction.
- [ ] Journal d'audit (`AuditEvent`, déjà défini dans
      `agoravote-core`) réellement émis par les handlers et persisté.
- [ ] Méthode de vote avancée supplémentaire — candidate proposée :
      jugement majoritaire (plus simple à tester que Condorcet/STV,
      cf. cahier des charges §2.2 pour la référence Condorcet PHP
      utilisable comme oracle de test).
- [ ] Export CSV/JSON dédié des résultats (§14 du cahier des charges) —
      aujourd'hui seul le JSON brut de l'API est disponible.
- [x] Câbler `agoravote-stats` (moyenne/médiane/écart-type) à une
      route API. Fait pour les questions nombre/échelle — cf.
      `docs/DEVLOG.md` itération 12 (`GET .../responses`, statistiques
      descriptives réelles). Reste à faire : tableau croisé, et
      utilisation de ces statistiques par l'écran Analyse du frontend
      (toujours sur donnée fictive pour cette partie).
- [ ] Câbler `voting.score` (implémenté et testé dans
      `agoravote-voting`, cf. `docs/DEVLOG.md` itération 12) à un
      chemin HTTP réel — aucun `QuestionType` actuel ne produit de
      bulletin `scores`, nécessiterait un type "notation par option"
      absent du modèle, décision produit avant travail technique.
- [ ] Attributs démographiques optionnels sur la participation (âge,
      territoire...), si le projet veut les comparaisons par groupe de
      la planche 6 — décision produit sur la collecte de ces données
      (cf. §13 "minimisation" du cahier des charges) avant tout travail
      technique, cf. `docs/DEVLOG.md` itération 7.

## Frontend

- [x] **Refonte sur les planches UX fournies** (nav latérale, tableau
      de bord, éditeur de formulaire, configuration du scrutin, vote
      citoyen, résultats en direct, écran Analyse) — fait, cf.
      `docs/DEVLOG.md` itération 7. A aussi corrigé deux bugs réels
      trouvés en repassant le parcours en navigateur (nav du haut
      cassée en mobile pour un utilisateur connecté ; nav latérale à
      largeur fixe rendant tout le contenu admin inutilisable en
      dessous de 1024px).
- [x] Types de question au-delà de choix unique/multiple dans le
      constructeur de formulaire (texte, nombre, échelle, classement —
      déjà supportés côté API). Fait — cf. `docs/DEVLOG.md` itération 7.
- [x] Vote et affichage des résultats pour texte/nombre/échelle/
      classement (le constructeur de formulaire de l'itération 7 les
      rendait créables mais pas votables — écran de vote sans UI de
      saisie, aucune méthode de dépouillement applicable). Fait — cf.
      `docs/DEVLOG.md` itération 12 : UI de saisie par type dans
      `Vote.tsx`, nouvelle distinction réponses brutes (`ResponsesView`/
      `ResponsesPanel`) vs résultat calculé (`TallyPanel`, choix
      unique/multiple uniquement).
- [ ] Écran de permissions/audit une fois le backend correspondant fait.
- [ ] Affichage multilingue de l'interface elle-même (le contenu des
      campagnes l'est déjà côté modèle — `prompt`/`labels` par langue
      — mais l'interface n'affiche que `fr`).
- [ ] Écran "Utilisateurs" et écran "Paramètres" (nav latérale
      `AdminLayout`, affichés grisés avec un badge "Bientôt" depuis
      l'itération 7, faute de backend — gestion d'utilisateurs,
      paramètres d'organisation) — attend les items backend
      correspondants (permissions fines, flux d'invitation).
- [ ] Écran "Analyse" (planche 6) plus complet — démographie
      (âge/territoire) et séries temporelles de participation,
      volontairement non construites en itération 7 faute de données
      réelles (cf. les deux items ci-dessous, préalables backend).

## Identité décentralisée (Ğ1v2)

Cf. `docs/G1_INTEGRATION.md` pour le détail complet de chaque étape.

- [x] Compiler `agoravote-g1` (cf. "Priorité immédiate" ci-dessus).
- [ ] Confirmer les noms de stockage réels (`Identity::IdentityIndexOf`,
      existence/nom d'un pallet `Membership` séparé) — préalable à
      câbler `chain-query` (vérification d'appartenance à la toile de
      confiance), toujours bloqué faute d'accès réseau à
      l'infrastructure Duniter dans tout environnement de développement
      utilisé jusqu'ici (reconfirmé en itération 8).
- [x] Implémenter le sketch d'intégration `agoravote-api` décrit dans
      `docs/G1_INTEGRATION.md` §4 (`POST /auth/g1/challenge`,
      `POST /auth/g1/verify`) et l'écran frontend `/connexion-g1`
      (planche 17 de l'addendum). Fait — cf. `docs/DEVLOG.md`
      itération 8. Ne prouve que la possession de clé, pas
      l'appartenance à la toile de confiance (cf. item précédent).
- [x] Documenter dans `docs/SECURITY.md` les implications d'un nœud
      RPC potentiellement malveillant. Fait — cf. `docs/SECURITY.md`
      §8 et `docs/DEVLOG.md` itération 8 (documenté par anticipation :
      `chain-query` n'est pas encore câblé, cf. item ci-dessus).
- [ ] **Reconfirmer le schéma de signature ed25519 contre une source
      primaire** (documentation officielle Duniter, ou test contre un
      vrai portefeuille Ğecko/Cesium²). Corrigé en itération 9 depuis
      un mauvais schéma initial (sr25519, jamais vérifié) sur la seule
      base de recherches web — `forum.duniter.org`/`git.duniter.org`
      restent bloqués par la liste blanche réseau de tout environnement
      utilisé jusqu'ici. Bloquant avant toute mise en production de ce
      module — cf. `docs/DEVLOG.md` itération 9, `docs/SECURITY.md` §8.
- [ ] Intégration d'une extension de portefeuille Ğ1 (Cesium²,
      Ğecko...) pour éviter le copier-coller manuel du défi/signature
      de l'écran `/connexion-g1` actuel — cf. `docs/DEVLOG.md`
      itération 8.
- [ ] Bascule d'activation de l'identité Ğ1 par organisation (écran
      "Paramètres" de la nav latérale, actuellement grisé) — aucune
      organisation ne peut aujourd'hui désactiver cette connexion.

## Sécurité et robustesse

- [ ] `cargo audit` en CI — la condition posée ici ("dès qu'une
      toolchain récente est disponible") est remplie depuis l'itération
      6 (`docs/DEVLOG.md`, rustc 1.94) mais le job reste à décommenter
      dans `.github/workflows/ci.yml` (cf. `docs/SECURITY.md` §1) ;
      pas fait cette itération, la CI elle-même n'a pas été modifiée.
- [ ] Limitation de débit (rate limiting) sur `/auth/login` et
      `/auth/register` — Argon2id ralentit le brute-force sur un mot
      de passe donné, pas les tentatives réparties sur des comptes
      différents.
- [ ] TLS géré par un reverse proxy devant les conteneurs, pour tout
      déploiement au-delà d'un usage local (`docker-compose.yml`
      actuel sert du HTTP en clair, pensé pour un réseau de confiance).

## Web3 / trajectoire long terme

Cf. l'addendum v0.3 du cahier des charges pour la vision complète —
non engageant à ce stade, à ne pas commencer avant que le module Ğ1
lecture-seule ci-dessus soit terminé et éprouvé.

- [ ] Étude d'ancrage vérifiable des `ResultSet` sur un registre public
      (empreinte, pas les données elles-mêmes).
