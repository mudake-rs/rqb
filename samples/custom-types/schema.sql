DROP SCHEMA IF EXISTS sample_custom CASCADE;
CREATE SCHEMA sample_custom;
CREATE DOMAIN sample_custom.cents AS bigint CHECK (VALUE >= 0);
CREATE TABLE sample_custom.wallets (id int PRIMARY KEY, balance sample_custom.cents NOT NULL);
