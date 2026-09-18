# Déploiement Docker — Zorin OS 18 et dérivés Debian/Ubuntu

Zorin OS 18 est basé sur Ubuntu (24.04 LTS) : tout ce qui suit
fonctionne tel quel sur Zorin, Ubuntu, Mint, ou toute distribution
Debian/Ubuntu récente.

## Installation en une commande

```bash
./install.sh
```

Ce script (cf. son en-tête pour le détail complet) :
1. vérifie si Docker Engine et le plugin `docker compose` sont
   installés ; sinon, propose de les installer via le dépôt officiel
   Docker (pas le paquet `docker.io` d'Ubuntu, souvent en retard) ;
2. construit l'image AgoraVote (`docker compose build`) ;
3. démarre le conteneur (`docker compose up -d`) ;
4. **attend activement** que `http://localhost:4000/health` réponde
   avant de vous rendre la main — il ne vous dit jamais "c'est prêt"
   par supposition.

Options : `./install.sh --yes` pour tout accepter sans confirmation
(utile en CI ou en script automatisé).

Si Docker n'est pas encore installé et que vous préférez le faire
vous-même avant de relancer `install.sh` :
<https://docs.docker.com/engine/install/ubuntu/>

## Vérifier que tout fonctionne

```bash
curl http://localhost:4000/health
# → ok

curl http://localhost:4000/modules
# → catalogue des méthodes de vote installées (voting.majority, ...)
```

Puis suivez le parcours de démonstration complet du `README.md`
(créer une campagne, voter, dépouiller) en remplaçant simplement
`BASE=http://localhost:4000` — c'est le même serveur que celui lancé
via `cargo run` (qui, lui, écoute par défaut sur le port 3000, cf.
section "Changer le port" ci-dessous pour la distinction entre port
hôte Docker et port du binaire).

Ouvrez ensuite **http://localhost:4080** dans un navigateur pour
l'interface web (cf. section "Frontend" ci-dessous).

## Frontend

Le service `frontend` (Nginx servant le build React, cf.
`frontend/Dockerfile`) est exposé par défaut sur le port hôte `4080`
(cf. `AGORAVOTE_FRONTEND_PORT`) et proxifie automatiquement `/api/*`
vers le service `api` — aucune configuration CORS à gérer, aucune
adresse d'API à saisir dans le navigateur.

```bash
curl -I http://localhost:4080/
# → 200 OK
```

Ce frontend a été construit et testé (compilation TypeScript, lint,
build de production) mais **jamais ouvert dans un vrai navigateur**
avant cette étape — c'est donc le tout premier test visuel réel du
projet. Si quelque chose semble cassé à l'écran, `docker compose logs
frontend` et la console développeur du navigateur (F12) sont les deux
premiers réflexes.

## Persistance PostgreSQL

Depuis l'introduction du crate `agoravote-store`, `docker-compose.yml`
démarre **deux** services : `db` (PostgreSQL 16) et `api`, connectés
automatiquement via `DATABASE_URL`. `install.sh` gère les deux
identiquement (`docker compose build`/`up` opèrent sur tout le
fichier).

**Les données survivent désormais à un redémarrage** :

```bash
docker compose restart   # ou down puis up -d
curl http://localhost:4000/campaigns/<id>   # la campagne est toujours là
```

Ce comportement a été vérifié pendant le développement : campagne +
bulletins créés, processus complètement arrêté, redémarré, relus à
l'identique via l'API (cf. `docs/DEVLOG.md`, itération 2).

**Changer le mot de passe PostgreSQL** (recommandé avant tout usage
au-delà de votre machine, cf. `docs/SECURITY.md`) :

```bash
echo 'POSTGRES_PASSWORD=un-mot-de-passe-solide' >> .env
docker compose down && docker compose up -d
```

**Réinitialiser complètement les données** (perte définitive,
confirmez avant de lancer) :

```bash
docker compose down -v   # -v supprime aussi le volume de données PostgreSQL
docker compose up -d     # repart sur une base vide
```

**Revenir temporairement au mode "sans base de données"** (utile pour
un test rapide sans vouloir gérer PostgreSQL) : lancez uniquement le
service `api` sans `DATABASE_URL`, en dehors de `docker-compose.yml` —
voir "Démarrer — sans Docker" dans le `README.md`, ou retirez
temporairement la ligne `DATABASE_URL` du service `api`.

## Logs

```bash
docker compose logs -f api
# ou, si vous avez installé Docker sans rejoindre le groupe 'docker' :
sudo docker compose logs -f api
```

Format et niveaux détaillés dans `docs/LOGGING.md`. Pour changer la
verbosité sans reconstruire l'image, créez un fichier `.env` à la
racine du projet :

```bash
echo 'RUST_LOG=debug' > .env
docker compose up -d   # redémarre avec la nouvelle valeur
```

## Arrêter / redémarrer

```bash
docker compose stop     # arrête sans supprimer le conteneur
docker compose start    # redémarre le conteneur arrêté
docker compose restart  # les deux d'un coup
```

Rappel : sans `DATABASE_URL` (mode sans base de données), toutes les
données restent en mémoire — un `restart` ou un `down` fait alors
repartir de zéro. Avec le `docker-compose.yml` fourni (PostgreSQL
activé par défaut), ce n'est plus le cas : voir la section
"Persistance PostgreSQL" ci-dessus.

## Désinstallation

```bash
./uninstall.sh              # retire le conteneur et le réseau, garde l'image en cache
./uninstall.sh --image      # retire aussi l'image agoravote-api:local
./uninstall.sh --help       # voir toutes les options
```

`uninstall.sh` **ne touche jamais à Docker Engine lui-même** par
défaut — seules les ressources propres à AgoraVote sont supprimées.
L'option `--purge-docker` existe pour aller plus loin, mais désinstalle
Docker de tout le système (donc pour tout autre projet Docker que vous
auriez) : elle demande une confirmation explicite séparée et n'est
utile que si vous n'avez plus aucun usage de Docker par ailleurs.

## Changer le port

Par défaut, l'API Docker est exposée sur `localhost:4000` et le
frontend sur `localhost:4080` — volontairement différents des ports
3000/8080 qu'on voit le plus souvent dans ce genre de projet, car
Docker Desktop (macOS/Linux) réserve couramment ces deux ports pour
son propre usage interne, ce qui provoquait `port is already
allocated` dès le premier démarrage sur certaines machines même sans
aucun autre service en conflit.

Si `4000` et/ou `4080` sont malgré tout déjà pris sur votre machine
(un autre projet Docker, par exemple), pas besoin de modifier
`docker-compose.yml` : créez un fichier `.env` à la racine du dépôt
(copie de `.env.example`, jamais committé) et fixez-y le port hôte de
votre choix, par exemple :

```bash
cp .env.example .env
```

```ini
# .env
AGORAVOTE_API_PORT=5000
AGORAVOTE_FRONTEND_PORT=5080
```

Puis relancez `docker compose up -d` (ou `./install.sh`) : l'API sera
accessible sur `http://localhost:5000` et le frontend sur
`http://localhost:5080`. Les ports *conteneur* (3000 pour l'API, 80
pour le frontend) ne changent jamais, eux — seul le port hôte, celui
que vous ouvrez dans un navigateur, est personnalisable.

## Dépannage

| Symptôme | Cause probable | Solution |
|---|---|---|
| `install.sh` échoue à "Le service Docker répond" | Le service Docker n'est pas démarré | `sudo systemctl start docker`, relancer `install.sh` |
| `permission denied` sur `docker` sans `sudo` | Votre utilisateur ne fait pas encore partie du groupe `docker` | Déconnectez-vous/reconnectez-vous après l'installation, ou `newgrp docker` |
| `port is already allocated` | Un autre programme (ou un autre projet Docker, ou Docker Desktop lui-même) utilise déjà le port 4000 et/ou 4080 sur votre machine | Fixez `AGORAVOTE_API_PORT`/`AGORAVOTE_FRONTEND_PORT` dans un fichier `.env` (voir "Changer le port" ci-dessus) — vérifiez aussi `docker ps -a` : un ancien conteneur (d'AgoraVote ou d'un autre projet) peut retenir le port même arrêté |
| Le conteneur redémarre en boucle | Voir les logs pour l'erreur exacte | `docker compose logs api` |
| `curl: (7) Failed to connect` juste après le démarrage | Le serveur met parfois 1-2s à démarrer | `install.sh` gère déjà cette attente ; en usage manuel, patientez puis réessayez |
| `api` reste "unhealthy" / redémarre en boucle, `db` semble démarré | `api` a démarré avant que `db` accepte des connexions | Normalement empêché par `depends_on: condition: service_healthy` ; vérifier `docker compose logs db` |
| Erreur de connexion PostgreSQL dans les logs `api` | Mot de passe désynchronisé entre `.env` et le volume déjà initialisé | PostgreSQL ne relit `POSTGRES_PASSWORD` qu'à la création du volume ; après un changement, `docker compose down -v` puis `up -d` (perte des données, cf. ci-dessus) |

## Ce que le `docker-compose.yml` ne fait pas (encore)

Un reverse proxy TLS (déploiement pensé pour un usage local/test, pas
une exposition publique directe) et le chiffrement au repos des
données PostgreSQL — voir `docs/SECURITY.md` pour la liste complète
des limites connues avant toute mise en production.
