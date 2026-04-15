.PHONY: fmt check-fmt clippy lint test check doc verify generate-schema db-up db-down db-reset

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
	RUSTFLAGS="-D warnings" cargo check --manifest-path samples/writes-and-types/Cargo.toml

generate-schema: docker-infra-up
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema public --out "$(GENERATED_SCHEMA)"
	rustfmt "$(GENERATED_SCHEMA)"

db-up: docker-infra-up

db-down: docker-infra-down

db-reset: docker-infra-reset

-include test/Makefile
