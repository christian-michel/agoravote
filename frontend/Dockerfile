# syntax=docker/dockerfile:1
#
# Même logique multi-étapes que le Dockerfile du backend (cf.
# ../Dockerfile) : une image de build Node (lourde) produit les
# fichiers statiques, une image Nginx minimale les sert — l'image
# finale ne contient ni Node, ni le code source, ni node_modules.
#
# Contexte de build = RACINE du dépôt (cf. docker-compose.yml,
# `context: .` + `dockerfile: frontend/Dockerfile`), pas `frontend/`
# seul : ce service a besoin de lire `docs/openapi.yaml`, qui vit hors
# du dossier `frontend/` — source de vérité unique du contrat d'API,
# partagée avec le backend (cf. `docs/openapi.yaml` lui-même).

FROM node:22-slim AS builder
WORKDIR /build

# Cache des dépendances : ne réinstalle que si package*.json change.
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

COPY frontend/. .
COPY docs/openapi.yaml /docs/openapi.yaml
RUN npx openapi-typescript /docs/openapi.yaml -o src/api/schema.ts \
    && npm run build

FROM nginx:1.27-alpine AS runtime
COPY --from=builder /build/dist /usr/share/nginx/html
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --quiet --spider http://localhost/ || exit 1
