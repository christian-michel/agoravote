# Feuille de route

Priorités actuelles, consolidées depuis les "prochaines itérations
candidates" de fin de chaque entrée de `docs/DEVLOG.md`. Ce fichier est
la référence unique et à jour — le DEVLOG garde l'historique des
décisions passées, celui-ci ne garde que ce qui reste à faire.

**Convention** : cocher un item ici quand il est fait, avec un renvoi
vers l'entrée DEVLOG correspondante. Ne jamais supprimer un item
coché — l'historique de ce qui a été priorisé et quand a de la valeur.

## Priorité immédiate

- [ ] **Vérifier le frontend dans un vrai navigateur.** Jamais fait
      jusqu'ici (pas de navigateur disponible pendant son
      développement) — `docker compose up` puis `http://localhost:8080`.
      Corriger ce qui ne va pas visuellement avant d'ajouter quoi que
      ce soit d'autre au frontend.
- [ ] **Réévaluer/compiler `agoravote-g1`** avec une toolchain à jour
      (cf. `CLAUDE.md`, section environnement). Si ça compile :
      confirmer les noms de stockage contre un nœud `gdev` réel (jamais
      `g1` en premier), ajouter un test avec un vecteur sr25519 connu,
      réintégrer au workspace principal.

## Backend

- [ ] `GET /campaigns` (liste, filtrée par organisation) — remplace le
      palliatif de mémorisation locale du tableau de bord frontend
      (`frontend/src/lib/recentCampaigns.ts`).
- [ ] Permissions fines : une campagne appartient à un organisateur
      précis (`owner_id`), pas à "tout Organizer de l'organisation".
- [ ] Flux d'invitation d'organisateurs — aujourd'hui, `/auth/register`
      attribue directement le rôle `Organizer` à quiconque s'inscrit.
- [ ] Validation bulletin ↔ type de question à la soumission (un
      bulletin `scores` sur une question `single_choice` n'est
      aujourd'hui filtré qu'au dépouillement, pas refusé à l'entrée).
- [ ] Journal d'audit (`AuditEvent`, déjà défini dans
      `agoravote-core`) réellement émis par les handlers et persisté.
- [ ] Méthode de vote avancée supplémentaire — candidate proposée :
      jugement majoritaire (plus simple à tester que Condorcet/STV,
      cf. cahier des charges §2.2 pour la référence Condorcet PHP
      utilisable comme oracle de test).
- [ ] Export CSV/JSON dédié des résultats (§14 du cahier des charges) —
      aujourd'hui seul le JSON brut de l'API est disponible.

## Frontend

- [ ] Types de question au-delà de choix unique/multiple dans le
      constructeur de formulaire (texte, nombre, échelle, classement —
      déjà supportés côté API).
- [ ] Écran de permissions/audit une fois le backend correspondant fait.
- [ ] Affichage multilingue de l'interface elle-même (le contenu des
      campagnes l'est déjà côté modèle — `prompt`/`labels` par langue
      — mais l'interface n'affiche que `fr`).

## Identité décentralisée (Ğ1v2)

Cf. `docs/G1_INTEGRATION.md` pour le détail complet de chaque étape.

- [ ] Compiler `agoravote-g1` (cf. "Priorité immédiate" ci-dessus).
- [ ] Confirmer les noms de stockage réels (`Identity::IdentityIndexOf`,
      existence/nom d'un pallet `Membership` séparé).
- [ ] Implémenter le sketch d'intégration `agoravote-api` décrit dans
      `docs/G1_INTEGRATION.md` §4 (`POST /auth/g1/challenge`,
      `POST /auth/g1/verify`).
- [ ] Documenter dans `docs/SECURITY.md` les implications d'un nœud
      RPC potentiellement malveillant (envisager l'interrogation de
      plusieurs nœuds indépendants pour une décision de légitimité de
      vote).

## Sécurité et robustesse

- [ ] `cargo audit` en CI dès qu'une toolchain récente est disponible
      (job prévu mais commenté dans `.github/workflows/ci.yml`,
      cf. `docs/SECURITY.md` §1).
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
