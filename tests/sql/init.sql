CREATE EXTENSION IF NOT EXISTS pgcrypto;

DROP VIEW IF EXISTS order_search_view;
DROP TABLE IF EXISTS events;
DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS app_users;
DROP TABLE IF EXISTS organizations;
DROP TYPE IF EXISTS order_status;
DROP TYPE IF EXISTS user_status;

CREATE TYPE user_status AS ENUM ('active', 'disabled');
CREATE TYPE order_status AS ENUM ('draft', 'paid', 'cancelled', 'refunded');

CREATE TABLE organizations (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    settings JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE app_users (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id),
    email TEXT NOT NULL UNIQUE,
    status user_status NOT NULL,
    profile JSONB NOT NULL DEFAULT '{}'::jsonb,
    tags TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE products (
    id UUID PRIMARY KEY,
    sku TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    price_cents BIGINT NOT NULL CHECK (price_cents >= 0),
    attributes JSONB NOT NULL DEFAULT '{}'::jsonb,
    tags TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE orders (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES app_users(id),
    status order_status NOT NULL,
    status_history order_status[] NOT NULL DEFAULT '{}',
    channel TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    tags TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE order_items (
    id UUID PRIMARY KEY,
    order_id UUID NOT NULL REFERENCES orders(id),
    product_id UUID NOT NULL REFERENCES products(id),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    unit_price_cents BIGINT NOT NULL CHECK (unit_price_cents >= 0),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE events (
    id UUID PRIMARY KEY,
    order_id UUID REFERENCES orders(id),
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_users_profile_gin ON app_users USING GIN (profile);
CREATE INDEX idx_users_tags ON app_users USING GIN (tags);
CREATE INDEX idx_products_attributes_gin ON products USING GIN (attributes);
CREATE INDEX idx_orders_metadata_gin ON orders USING GIN (metadata);
CREATE INDEX idx_orders_tags ON orders USING GIN (tags);
CREATE INDEX idx_orders_created_at ON orders (created_at DESC);
CREATE INDEX idx_events_payload_gin ON events USING GIN (payload);

INSERT INTO organizations (id, slug, name, settings, created_at) VALUES
('00000000-0000-0000-0000-000000000001', 'acme', 'Acme', '{"tier":"enterprise"}', '2026-01-01T00:00:00Z'),
('00000000-0000-0000-0000-000000000002', 'globex', 'Globex', '{"tier":"startup"}', '2026-01-02T00:00:00Z');

INSERT INTO app_users (id, organization_id, email, status, profile, tags, created_at) VALUES
('10000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001', 'ada@example.com', 'active', '{"country":"NL","score":98}', ARRAY['vip','beta'], '2026-01-03T00:00:00Z'),
('10000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000001', 'grace@example.com', 'active', '{"country":"US","score":80}', ARRAY['standard'], '2026-01-04T00:00:00Z'),
('10000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000002', 'linus@example.com', 'disabled', '{"country":"FI","score":70}', ARRAY['vip'], '2026-01-05T00:00:00Z');

INSERT INTO products (id, sku, name, price_cents, attributes, tags, created_at) VALUES
('20000000-0000-0000-0000-000000000001', 'CAM-001', 'Camera', 10900, '{"color":"black","weight":1.3}', ARRAY['hardware','photo'], '2026-01-06T00:00:00Z'),
('20000000-0000-0000-0000-000000000002', 'BAG-001', 'Bag', 5000, '{"color":"green","weight":0.4}', ARRAY['accessory'], '2026-01-07T00:00:00Z'),
('20000000-0000-0000-0000-000000000003', 'MIC-001', 'Microphone', 7000, '{"color":"black","weight":0.6}', ARRAY['hardware','audio'], '2026-01-08T00:00:00Z');

INSERT INTO orders (id, user_id, status, status_history, channel, metadata, tags, created_at) VALUES
('30000000-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000001', 'paid', ARRAY['draft','paid']::order_status[], 'web', '{"score":92,"gift":true,"campaign":"spring"}', ARRAY['vip','gift'], '2026-02-01T10:00:00Z'),
('30000000-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000002', 'paid', ARRAY['draft','paid']::order_status[], 'mobile', '{"score":45,"gift":false,"campaign":"winter"}', ARRAY['standard'], '2026-02-02T10:00:00Z'),
('30000000-0000-0000-0000-000000000003', '10000000-0000-0000-0000-000000000001', 'draft', ARRAY['draft']::order_status[], 'web', '{"score":15,"gift":false,"campaign":"spring"}', ARRAY['draft'], '2026-02-03T10:00:00Z'),
('30000000-0000-0000-0000-000000000004', '10000000-0000-0000-0000-000000000003', 'paid', ARRAY['draft','paid']::order_status[], 'partner', '{"score":88,"gift":true,"campaign":"spring"}', ARRAY['vip','partner'], '2025-12-31T10:00:00Z');

INSERT INTO order_items (id, order_id, product_id, quantity, unit_price_cents, metadata) VALUES
('40000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000001', 1, 10900, '{"warehouse":"ams"}'),
('40000000-0000-0000-0000-000000000002', '30000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000002', 1, 5000, '{"warehouse":"ams"}'),
('40000000-0000-0000-0000-000000000003', '30000000-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000003', 1, 7000, '{"warehouse":"nyc"}'),
('40000000-0000-0000-0000-000000000004', '30000000-0000-0000-0000-000000000003', '20000000-0000-0000-0000-000000000002', 1, 5000, '{"warehouse":"ams"}'),
('40000000-0000-0000-0000-000000000005', '30000000-0000-0000-0000-000000000004', '20000000-0000-0000-0000-000000000001', 1, 10900, '{"warehouse":"hel"}');

INSERT INTO events (id, order_id, event_type, payload, created_at) VALUES
('50000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000001', 'paid', '{"gateway":"stripe","risk":12}', '2026-02-01T10:01:00Z'),
('50000000-0000-0000-0000-000000000002', '30000000-0000-0000-0000-000000000002', 'paid', '{"gateway":"adyen","risk":60}', '2026-02-02T10:01:00Z'),
('50000000-0000-0000-0000-000000000003', '30000000-0000-0000-0000-000000000003', 'created', '{"gateway":null,"risk":0}', '2026-02-03T10:01:00Z');

CREATE VIEW order_search_view AS
SELECT
    o.id,
    u.email,
    u.organization_id,
    o.status,
    o.status_history,
    o.channel,
    o.tags,
    o.metadata,
    o.created_at,
    COUNT(oi.id)::BIGINT AS items_count,
    COALESCE(SUM(oi.quantity * oi.unit_price_cents), 0)::BIGINT AS total_cents
FROM orders o
JOIN app_users u ON u.id = o.user_id
LEFT JOIN order_items oi ON oi.order_id = o.id
GROUP BY o.id, u.email, u.organization_id, o.status, o.status_history, o.channel, o.tags, o.metadata, o.created_at;
