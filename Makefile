# Commandes de développement standardisées — cf. docs/DEVELOPMENT_WORKFLOW.md
#
# Objectif : qu'un contributeur (ou vous-même dans six mois) n'ait
# jamais à deviner la bonne commande cargo à taper. `make check` fait
# exactement ce que la CI fait, dans le même ordre — voir
# .github/workflows/ci.yml.

.PHONY: help check fmt fmt-check lint test run docker-build docker-up docker-down clean

help: ## Affiche cette aide
	@grep -E '^[a-zA-Z_-]+:.*## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

check: fmt-check lint test ## Lance tout ce que la CI vérifie (à faire avant tout commit)

fmt: ## Reformate le code (modifie les fichiers)
	cargo fmt

fmt-check: ## Vérifie le formatage sans rien modifier (utilisé par la CI)
	cargo fmt --check

lint: ## Lance clippy en mode strict (warnings = erreurs)
	cargo clippy --workspace --all-targets -- -D warnings

test: ## Lance tous les tests du workspace
	cargo test --workspace

run: ## Démarre l'API en local avec des logs verbeux
	RUST_LOG=info,agoravote_api=debug,agoravote_core=debug,agoravote_voting=debug,agoravote_stats=debug,tower_http=debug \
		cargo run -p agoravote-api

docker-build: ## Construit l'image Docker de l'API
	docker compose build

docker-up: ## Démarre AgoraVote via Docker (arrière-plan)
	docker compose up -d
	@echo "API disponible sur http://localhost:3000 — logs : make docker-logs"

docker-down: ## Arrête et retire les conteneurs Docker
	docker compose down

docker-logs: ## Suit les logs du conteneur API
	docker compose logs -f api

clean: ## Nettoie les artefacts de build Rust
	cargo clean
