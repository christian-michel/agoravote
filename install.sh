#!/usr/bin/env bash
# install.sh — installe et démarre AgoraVote via Docker.
#
# Ciblé pour Zorin OS 18 (basé sur Ubuntu 22.04/24.04) mais fonctionne
# sur toute distribution Debian/Ubuntu avec apt. Ce script :
#   1. vérifie la présence de Docker et du plugin `docker compose` ;
#      les installe (avec votre accord) si absents ;
#   2. construit l'image AgoraVote ;
#   3. démarre le conteneur ;
#   4. attend que /health réponde réellement avant de rendre la main,
#      pour ne jamais vous dire "c'est prêt" alors que ça ne l'est pas
#      encore (cf. docs/LOGGING.md — même philosophie que le
#      HEALTHCHECK du Dockerfile : on VÉRIFIE, on ne suppose pas).
#
# Usage :
#   ./install.sh            # interactif, demande confirmation si Docker manque
#   ./install.sh --yes      # non-interactif, répond "oui" à tout
#
# cf. docs/DOCKER.md pour le détail et le dépannage.

set -euo pipefail

# --- Couleurs (désactivées si la sortie n'est pas un terminal) -----------
if [ -t 1 ]; then
    C_RESET=$'\033[0m'; C_BOLD=$'\033[1m'; C_GREEN=$'\033[32m'
    C_YELLOW=$'\033[33m'; C_RED=$'\033[31m'; C_BLUE=$'\033[34m'
else
    C_RESET=""; C_BOLD=""; C_GREEN=""; C_YELLOW=""; C_RED=""; C_BLUE=""
fi

info()  { printf '%s[i]%s %s\n' "$C_BLUE" "$C_RESET" "$1"; }
ok()    { printf '%s[✓]%s %s\n' "$C_GREEN" "$C_RESET" "$1"; }
warn()  { printf '%s[!]%s %s\n' "$C_YELLOW" "$C_RESET" "$1"; }
error() { printf '%s[✗]%s %s\n' "$C_RED" "$C_RESET" "$1" >&2; }

ASSUME_YES=false
for arg in "$@"; do
    case "$arg" in
        --yes|-y) ASSUME_YES=true ;;
        --help|-h)
            echo "Usage: $0 [--yes]"
            echo "  --yes  Ne pose aucune question (installe Docker si besoin sans confirmation)."
            exit 0
            ;;
        *)
            error "Option inconnue : $arg (voir --help)"
            exit 1
            ;;
    esac
done

confirm() {
    # $1 = question posée. Renvoie 0 (succès) si l'utilisateur accepte.
    if [ "$ASSUME_YES" = true ]; then
        return 0
    fi
    read -r -p "$1 [o/N] " reponse
    case "$reponse" in
        o|O|oui|y|Y|yes) return 0 ;;
        *) return 1 ;;
    esac
}

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
cd "$SCRIPT_DIR"

echo "${C_BOLD}=== Installation d'AgoraVote ===${C_RESET}"
echo

# `docker compose` lit automatiquement un `.env` à la racine pour
# résoudre les `${...}` de `docker-compose.yml` (AGORAVOTE_API_PORT,
# AGORAVOTE_FRONTEND_PORT — cf. .env.example et docs/DOCKER.md,
# "Changer le port") ; ce script bash, lui, ne le fait pas
# automatiquement — on le source ici pour que les URLs affichées
# ci-dessous correspondent réellement aux ports utilisés, y compris
# quand ils ont été personnalisés pour éviter un conflit.
if [ -f .env ]; then
    set -a
    # shellcheck disable=SC1091
    source .env
    set +a
fi
API_PORT="${AGORAVOTE_API_PORT:-3000}"
FRONTEND_PORT="${AGORAVOTE_FRONTEND_PORT:-8080}"

# --- 1. Docker Engine ------------------------------------------------------
if command -v docker &>/dev/null; then
    ok "Docker est déjà installé ($(docker --version))."
else
    warn "Docker n'est pas installé."
    if confirm "Installer Docker Engine maintenant via apt (nécessite sudo) ?"; then
        info "Installation de Docker Engine (dépôt officiel Docker)..."
        # Méthode recommandée par Docker pour Ubuntu/Debian (donc Zorin
        # OS, qui en dérive) plutôt que le paquet `docker.io` d'Ubuntu,
        # souvent en retard de plusieurs versions.
        sudo apt-get update
        sudo apt-get install -y ca-certificates curl gnupg
        sudo install -m 0755 -d /etc/apt/keyrings
        curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
        sudo chmod a+r /etc/apt/keyrings/docker.gpg
        # shellcheck disable=SC1091
        . /etc/os-release
        echo \
            "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
            ${UBUNTU_CODENAME:-$VERSION_CODENAME} stable" | \
            sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
        sudo apt-get update
        sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

        if ! groups "$USER" | grep -q docker; then
            info "Ajout de $USER au groupe 'docker' (évite de préfixer chaque commande par sudo)..."
            sudo usermod -aG docker "$USER"
            warn "Vous devez vous déconnecter/reconnecter (ou lancer 'newgrp docker') pour que ce changement prenne effet."
            warn "En attendant, ce script continue avec 'sudo docker'."
            SUDO_DOCKER=true
        fi
        ok "Docker Engine installé."
    else
        error "Docker est requis. Installation annulée."
        echo "Documentation officielle : https://docs.docker.com/engine/install/ubuntu/"
        exit 1
    fi
fi

SUDO_DOCKER="${SUDO_DOCKER:-false}"
DOCKER_CMD="docker"
if [ "$SUDO_DOCKER" = true ]; then
    DOCKER_CMD="sudo docker"
fi

# --- 2. Plugin docker compose ----------------------------------------------
if $DOCKER_CMD compose version &>/dev/null; then
    ok "Le plugin 'docker compose' est disponible ($($DOCKER_CMD compose version --short 2>/dev/null || echo 'version inconnue'))."
else
    error "Le plugin 'docker compose' (v2) est introuvable, alors que Docker l'est."
    echo "Sur Zorin OS/Ubuntu : sudo apt-get install docker-compose-plugin"
    exit 1
fi

# --- 3. Le service Docker tourne-t-il ? -------------------------------------
if ! $DOCKER_CMD info &>/dev/null; then
    error "Docker est installé mais le service ne répond pas (ou vous n'avez pas les droits)."
    echo "Essayez : sudo systemctl start docker"
    echo "Puis relancez ce script. Si le problème persiste après avoir rejoint le"
    echo "groupe 'docker', déconnectez-vous et reconnectez-vous à votre session."
    exit 1
fi
ok "Le service Docker répond."

# --- 4. Construction de l'image ---------------------------------------------
echo
info "Construction de l'image AgoraVote (peut prendre quelques minutes la première fois)..."
$DOCKER_CMD compose build

# --- 5. Démarrage -------------------------------------------------------------
info "Démarrage du conteneur..."
$DOCKER_CMD compose up -d

# --- 6. Attente active du /health --------------------------------------------
info "Attente que l'API réponde sur /health..."
MAX_ATTEMPTS=30
attempt=0
until curl --fail --silent --output /dev/null "http://localhost:${API_PORT}/health" 2>/dev/null; do
    attempt=$((attempt + 1))
    if [ "$attempt" -ge "$MAX_ATTEMPTS" ]; then
        error "L'API ne répond toujours pas après ${MAX_ATTEMPTS} tentatives (~${MAX_ATTEMPTS}0s)."
        echo "Voir les logs pour comprendre pourquoi :"
        echo "  $DOCKER_CMD compose logs api"
        exit 1
    fi
    sleep 1
done

echo
ok "AgoraVote est démarré et répond correctement."
echo
echo "  ${C_BOLD}Application${C_RESET} : http://localhost:${FRONTEND_PORT}"
echo "  ${C_BOLD}API${C_RESET}         : http://localhost:${API_PORT}"
echo "  ${C_BOLD}Santé${C_RESET}       : http://localhost:${API_PORT}/health"
echo "  ${C_BOLD}Modules${C_RESET}     : http://localhost:${API_PORT}/modules"
echo "  ${C_BOLD}Logs${C_RESET}        : $DOCKER_CMD compose logs -f api"
echo "  ${C_BOLD}Arrêt${C_RESET}       : $DOCKER_CMD compose down"
echo
echo "Voir README.md pour un parcours de démonstration complet (créer une"
echo "campagne, voter, dépouiller) et docs/DOCKER.md pour le dépannage."
