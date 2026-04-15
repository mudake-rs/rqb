.PHONY: fmt check-fmt clippy lint test check doc verify generate-schema generate-sample-schema generate-sample-schemas db-up db-down db-reset

DATABASE_URL ?= postgres://rqb:rqb@localhost:55432/rqb
GENERATED_SCHEMA ?= target/generated/rqb_schema.rs

fmt:
	cargo fmt --all

check-fmt:
	cargo fmt --all --check

clippy:
	cargo clippy --workspace --all-targets --no-default-features -- -D warnings

lint: check-fmt clippy

test:
	cargo test --workspace --no-default-features

check: lint test

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-default-features --no-deps

verify: check doc
	RUSTFLAGS="-D warnings" cargo check --manifest-path samples/basic-queries/Cargo.toml
	RUSTFLAGS="-D warnings" cargo check --manifest-path samples/json-search/Cargo.toml
	RUSTFLAGS="-D warnings" cargo check --manifest-path samples/rest-api/Cargo.toml
	RUSTFLAGS="-D warnings" cargo check --manifest-path samples/writes-and-types/Cargo.toml

generate-schema: docker-infra-up
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema public --out "$(GENERATED_SCHEMA)"
	rustfmt "$(GENERATED_SCHEMA)"

generate-sample-schema: generate-sample-schemas

generate-sample-schemas: docker-infra-up
	docker compose -f test/docker-compose.yaml exec -T postgres psql -v ON_ERROR_STOP=1 -U rqb -d rqb < samples/schema.sql
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema sample --table app_users --out samples/basic-queries/src/schema.rs
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema sample --table order_search_view --out samples/json-search/src/schema.rs
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema sample --table app_users --table orders --out samples/rest-api/src/schema.rs
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema sample --table invoices --out samples/writes-and-types/src/schema.rs
	rustfmt samples/basic-queries/src/schema.rs samples/json-search/src/schema.rs samples/rest-api/src/schema.rs samples/writes-and-types/src/schema.rs

db-up: docker-infra-up

db-down: docker-infra-down

db-reset: docker-infra-reset

-include test/Makefile
