.PHONY: docker-infra-up docker-infra-down docker-infra-reset test-integration integration-test docker-test

DOCKER_TEST_COMPOSE = docker compose -p rqb_test -f test/docker-compose.test.yaml
DOCKER_DEV_COMPOSE = docker compose -p rqb_dev -f test/docker-compose.yaml

docker-infra-up:
	$(DOCKER_DEV_COMPOSE) up -d --wait postgres

docker-infra-down:
	$(DOCKER_DEV_COMPOSE) down

docker-infra-reset:
	$(DOCKER_DEV_COMPOSE) down -v
	$(DOCKER_DEV_COMPOSE) up -d --wait postgres

test-integration: docker-infra-up
	RQB_TEST_DATABASE_URL="$(DATABASE_URL)" cargo test -p rqb --test postgres_integration --all-features -- --ignored --test-threads=1
	RQB_TEST_DATABASE_URL="$(DATABASE_URL)" cargo test -p rqb-cli -- --ignored

.PHONY: test-sample-integration
test-sample-integration: docker-infra-up
	$(DOCKER_DEV_COMPOSE) exec -T postgres psql -v ON_ERROR_STOP=1 -U rqb -d rqb < samples/schema.sql
	$(DOCKER_DEV_COMPOSE) exec -T postgres psql -v ON_ERROR_STOP=1 -U rqb -d rqb < samples/custom-types/schema.sql
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema sample --out samples/schema/src/lib.rs --check
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema sample_custom --config samples/custom-types/rqb.toml --out samples/custom-types/src/mapped_schema.rs --check
	DATABASE_URL="$(DATABASE_URL)" cargo test --manifest-path samples/schema/Cargo.toml -- --ignored
	RQB_TEST_DATABASE_URL="$(DATABASE_URL)" cargo test --manifest-path samples/custom-types/Cargo.toml -- --ignored
	RQB_TEST_DATABASE_URL="$(DATABASE_URL)" cargo test --manifest-path samples/joins-and-aggregates/Cargo.toml -- --ignored

integration-test: test-integration

docker-test:
	@set -e; \
	cleanup() { $(DOCKER_TEST_COMPOSE) down --remove-orphans; }; \
	trap 'status=$$?; cleanup; exit $$status' EXIT INT TERM; \
	$(DOCKER_TEST_COMPOSE) build tests; \
	$(DOCKER_TEST_COMPOSE) up -d --wait postgres; \
	$(DOCKER_TEST_COMPOSE) run --rm --no-deps tests
