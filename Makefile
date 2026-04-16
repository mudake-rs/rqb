.PHONY: fmt check-fmt clippy lint test check doc verify generate-schema generate-sample-schema generate-sample-schemas db-up db-down db-reset

DATABASE_URL ?= postgres://rqb:rqb@localhost:55432/rqb
GENERATED_SCHEMA ?= target/generated/rqb_schema.rs

fmt:
	cargo fmt --all

check-fmt:
	cargo fmt --all --check

clippy:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

lint: check-fmt clippy

test:
	cargo test --workspace --all-features

check: lint test

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps

verify: check doc
	RUSTFLAGS="-D warnings" cargo check --manifest-path samples/schema/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/basic-queries/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/json-search/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/writes-and-types/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/transactions/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/error-handling/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/raw-query/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/joins-and-aggregates/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/cte-and-subqueries/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/advanced-queries/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/custom-types/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/rest-api/Cargo.toml

generate-schema: docker-infra-up
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema public --out "$(GENERATED_SCHEMA)"
	rustfmt "$(GENERATED_SCHEMA)"

generate-sample-schema: generate-sample-schemas

generate-sample-schemas: docker-infra-up
	docker compose -f test/docker-compose.yaml exec -T postgres psql -v ON_ERROR_STOP=1 -U rqb -d rqb < samples/schema.sql
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema sample --out samples/schema/src/lib.rs
	rustfmt samples/schema/src/lib.rs

db-up: docker-infra-up

db-down: docker-infra-down

db-reset: docker-infra-reset

-include test/Makefile
