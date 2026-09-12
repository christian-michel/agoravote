#!/usr/bin/env bash
# uninstall.sh — arrête et retire proprement les ressources Docker
# créées par AgoraVote (conteneur, réseau, image).
#
# Par défaut, NE TOUCHE PAS à Docker Engine lui-même (d'autres projets
# sur votre machine peuvent en dépendre) : seules les ressources
# propres à AgoraVote sont supprimées. Voir --purge-docker pour aller
# plus loin, avec un avertissement explicite avant toute action.
#
# Usage :
#   ./uninstall.sh                 # retire conteneur + réseau (garde l'image)
#   ./uninstall.sh --image         # retire aussi l'image agoravote-api:local
#   ./uninstall.sh --yes           # ne pose aucune question
#   ./uninstall.sh --purge-docker  # (dangereux) désinstalle Docker Engine du système

set -euo pipefail

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
REMOVE_IMAGE=false
PURGE_DOCKER=false

for arg in "$@"; do
    case "$arg" in
        --yes|-y) ASSUME_YES=true ;;
        --image) REMOVE_IMAGE=true ;;
        --purge-docker) PURGE_DOCKER=true ;;
        --help|-h)
            echo "Usage: $0 [--image] [--purge-docker] [--yes]"
            echo "  --image          Retire aussi l'image Docker agoravote-api:local."
            echo "  --purge-docker   DANGEREUX : désinstalle Docker Engine du système entier"
            echo "                   (affecte tout autre projet Docker sur cette machine)."
            echo "  --yes            Ne pose aucune question."
            exit 0
            ;;
        *)
            error "Option inconnue : $arg (voir --help)"
            exit 1
            ;;
    esac
done

confirm() {
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

echo "${C_BOLD}=== Désinstallation d'AgoraVote ===${C_RESET}"
echo

if ! command -v docker &>/dev/null; then
    warn "Docker n'est pas installé — rien à désinstaller pour AgoraVote."
    exit 0
fi

DOCKER_CMD="docker"
if ! docker info &>/dev/null; then
    DOCKER_CMD="sudo docker"
fi

# --- 1. Arrêt et retrait du conteneur + réseau ------------------------------
info "Arrêt et retrait du conteneur AgoraVote..."
if $DOCKER_CMD compose ps --quiet 2>/dev/null | grep -q .; then
    $DOCKER_CMD compose down
    ok "Conteneur et réseau retirés."
else
    warn "Aucun conteneur AgoraVote en cours d'exécution (rien à arrêter)."
    # `down` reste utile même sans conteneur actif : il nettoie un
    # éventuel réseau orphelin laissé par une exécution précédente.
    $DOCKER_CMD compose down --remove-orphans 2>/dev/null || true
fi

# --- 2. Image Docker (optionnel) --------------------------------------------
if [ "$REMOVE_IMAGE" = true ]; then
    if $DOCKER_CMD image inspect agoravote-api:local &>/dev/null; then
        if confirm "Retirer l'image agoravote-api:local (elle sera reconstruite au prochain install.sh) ?"; then
            $DOCKER_CMD image rm agoravote-api:local
            ok "Image retirée."
        fi
    else
        warn "Aucune image agoravote-api:local trouvée."
    fi
else
    info "Image Docker conservée (relancez avec --image pour la retirer aussi)."
fi

# --- 3. Vérification qu'il ne reste rien -----------------------------------
REMAINING=$($DOCKER_CMD ps -a --filter "name=agoravote-api" --format "{{.Names}}" 2>/dev/null || true)
if [ -n "$REMAINING" ]; then
    warn "Attention : une ressource nommée 'agoravote-api' existe encore : $REMAINING"
    warn "Vérifiez manuellement avec : $DOCKER_CMD ps -a"
else
    ok "Aucune ressource AgoraVote restante (conteneur/réseau)."
fi

# --- 4. Purge complète de Docker Engine (optionnel, dangereux) -------------
if [ "$PURGE_DOCKER" = true ]; then
    echo
    warn "Vous avez demandé --purge-docker : ceci va désinstaller Docker Engine"
    warn "DU SYSTÈME ENTIER, ce qui affectera TOUT AUTRE projet utilisant Docker"
    warn "sur cette machine, pas seulement AgoraVote."
    if confirm "${C_RED}Confirmer la désinstallation complète de Docker Engine ?${C_RESET}"; then
        sudo apt-get purge -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
        sudo apt-get autoremove -y
        if confirm "Supprimer aussi les images/volumes Docker restants sur le système (/var/lib/docker) ?"; then
            sudo rm -rf /var/lib/docker /var/lib/containerd
            ok "Données Docker supprimées."
        fi
        ok "Docker Engine désinstallé."
    else
        info "Purge de Docker annulée."
    fi
fi

echo
ok "Désinstallation d'AgoraVote terminée."
