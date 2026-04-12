.PHONY: fmt check-fmt clippy lint test check test-integration verify generate-demo generate-sample-schema db-up db-down db-reset

DATABASE_URL ?= postgres://rqb:rqb@localhost:55432/rqb
GENERATED_SCHEMA ?= target/generated/rqb_schema.rs
SAMPLE_SCHEMA ?= samples/rest-api/src/db/schema.rs

fmt:
	cargo fmt --all

check-fmt:
	cargo fmt --all --check

clippy:
	cargo clippy --all-targets --all-features --workspace -- -D warnings

lint: check-fmt clippy

test:
	cargo test --workspace

check: lint test

test-integration: docker-infra-up
	RQB_TEST_DATABASE_URL="$(DATABASE_URL)" cargo test -p rqb-postgres --features runtime-tokio-postgres --test postgres_integration -- --nocapture

verify: docker-infra-up
	cargo build --workspace
	cargo test --workspace
	RQB_TEST_DATABASE_URL="$(DATABASE_URL)" cargo test -p rqb-postgres --features pool --test postgres_integration -- --nocapture

generate-demo: docker-infra-up
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema public --out "$(GENERATED_SCHEMA)"
	rustfmt "$(GENERATED_SCHEMA)"
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema public --out "$(SAMPLE_SCHEMA)"
	rustfmt "$(SAMPLE_SCHEMA)"

generate-sample-schema: docker-infra-up
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema public --out "$(SAMPLE_SCHEMA)"
	rustfmt "$(SAMPLE_SCHEMA)"

db-up: docker-infra-up

db-down: docker-infra-down

db-reset: docker-infra-reset

-include test/Makefile
