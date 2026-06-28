.PHONY: help fmt check-fmt clippy lint test check doc verify verify-full generate-schema generate-sample-schema generate-sample-schemas smoke-rqb-cli db-up db-down db-reset

DATABASE_URL ?= postgres://rqb:rqb@localhost:55432/rqb
GENERATED_SCHEMA ?= target/generated/rqb_schema.rs

help:
	@printf "Common targets:\n"
	@printf "  make fmt                    Format workspace\n"
	@printf "  make check                  fmt check + clippy + tests\n"
	@printf "  make verify                 Full local no-DB verification + samples\n"
	@printf "  make test-integration       Run ignored Postgres 18 integration tests\n"
	@printf "  make docker-test            Run full Docker-backed test suite\n"
	@printf "  make db-up/db-down/db-reset Manage local Postgres test DB\n"
	@printf "  make generate-schema        Generate schema from public test DB schema\n"
	@printf "  make generate-sample-schema Regenerate samples/schema from sample schema\n"
	@printf "  make smoke-rqb-cli          Smoke-test rqb-cli against the sample schema\n"

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
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/query-reuse-and-pagination/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/writes-and-types/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/transactions/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/executor-wrapper/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/error-handling/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/raw-query/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/joins-and-aggregates/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/cte-and-subqueries/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/advanced-queries/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/custom-types/Cargo.toml
	RUSTFLAGS="-D warnings" cargo run --manifest-path samples/rest-api/Cargo.toml

verify-full: verify test-integration

generate-schema: docker-infra-up
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema public --out "$(GENERATED_SCHEMA)"

generate-sample-schema: generate-sample-schemas

generate-sample-schemas: docker-infra-up
	docker compose -f test/docker-compose.yaml exec -T postgres psql -v ON_ERROR_STOP=1 -U rqb -d rqb < samples/schema.sql
	cargo run -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema sample --out samples/schema/src/lib.rs

smoke-rqb-cli: docker-infra-up
	docker compose -f test/docker-compose.yaml exec -T postgres psql -v ON_ERROR_STOP=1 -U rqb -d rqb < samples/schema.sql
	cargo run -q -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema sample --out /tmp/_rqb_cli_smoke.rs
	cargo run -q -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema sample --stdout > /tmp/_rqb_cli_smoke_stdout.rs
	diff /tmp/_rqb_cli_smoke.rs /tmp/_rqb_cli_smoke_stdout.rs
	cargo run -q -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema sample --out /tmp/_rqb_cli_smoke.rs --check
	printf "\n// drift\n" >> /tmp/_rqb_cli_smoke.rs
	! cargo run -q -p rqb-cli -- generate --database-url "$(DATABASE_URL)" --schema sample --out /tmp/_rqb_cli_smoke.rs --check
	rm -f /tmp/_rqb_cli_smoke.rs /tmp/_rqb_cli_smoke_stdout.rs

db-up: docker-infra-up

db-down: docker-infra-down

db-reset: docker-infra-reset

-include test/Makefile
