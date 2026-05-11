.PHONY: docker-infra-up docker-infra-down docker-infra-reset test-integration integration-test docker-test

DOCKER_TEST_COMPOSE = docker compose -p rqb_test -f test/docker-compose.test.yaml

docker-infra-up:
	docker compose -f test/docker-compose.yaml up -d --wait postgres

docker-infra-down:
	docker compose -f test/docker-compose.yaml down

docker-infra-reset:
	docker compose -f test/docker-compose.yaml down -v
	docker compose -f test/docker-compose.yaml up -d --wait postgres

test-integration: docker-infra-up
	RQB_TEST_DATABASE_URL="$(DATABASE_URL)" cargo test -p rqb --test postgres_integration --all-features -- --ignored --test-threads=1

integration-test: test-integration

docker-test:
	@set -e; \
	cleanup() { $(DOCKER_TEST_COMPOSE) down --remove-orphans; }; \
	trap 'status=$$?; cleanup; exit $$status' EXIT INT TERM; \
	$(DOCKER_TEST_COMPOSE) build tests; \
	$(DOCKER_TEST_COMPOSE) up -d --wait postgres; \
	$(DOCKER_TEST_COMPOSE) run --rm --no-deps tests
