.PHONY: docker-infra-up docker-infra-down docker-infra-reset docker-test

docker-infra-up:
	docker compose -f test/docker-compose.yaml up -d --wait postgres

docker-infra-down:
	docker compose -f test/docker-compose.yaml down

docker-infra-reset:
	docker compose -f test/docker-compose.yaml down -v
	docker compose -f test/docker-compose.yaml up -d --wait postgres

docker-test:
	docker compose -p rqb_test -f test/docker-compose.test.yaml up -d --no-recreate postgres
	docker compose -p rqb_test -f test/docker-compose.test.yaml run --rm --no-deps tests; \
	status=$$?; \
	docker compose -p rqb_test -f test/docker-compose.test.yaml stop; \
	exit $$status
