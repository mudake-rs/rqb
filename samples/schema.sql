DROP SCHEMA IF EXISTS sample CASCADE;
CREATE SCHEMA sample;

CREATE TYPE sample.invoice_state AS ENUM ('draft', 'issued', 'paid', 'void');

CREATE TABLE sample.organizations (
    id uuid CONSTRAINT organizations_pkey PRIMARY KEY,
    slug text NOT NULL,
    name text NOT NULL,
    settings jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT organizations_slug_key UNIQUE (slug),
    CONSTRAINT organizations_slug_format CHECK (slug ~ '^[a-z0-9][a-z0-9-]*$'),
    CONSTRAINT organizations_settings_object CHECK (jsonb_typeof(settings) = 'object')
);

CREATE TABLE sample.app_users (
    id uuid CONSTRAINT app_users_pkey PRIMARY KEY,
    organization_id uuid,
    email text NOT NULL,
    status text NOT NULL DEFAULT 'active',
    display_name text NOT NULL,
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT app_users_organization_fkey
        FOREIGN KEY (organization_id) REFERENCES sample.organizations(id)
        ON DELETE SET NULL,
    CONSTRAINT app_users_email_key UNIQUE (email),
    CONSTRAINT app_users_organization_email_key UNIQUE (organization_id, email),
    CONSTRAINT app_users_status_check CHECK (status IN ('invited', 'active', 'disabled')),
    CONSTRAINT app_users_display_name_not_blank CHECK (length(trim(display_name)) > 0)
);

CREATE TABLE sample.products (
    id uuid CONSTRAINT products_pkey PRIMARY KEY,
    sku text NOT NULL,
    name text NOT NULL,
    price_cents bigint NOT NULL,
    attributes jsonb NOT NULL DEFAULT '{}'::jsonb,
    tags text[] NOT NULL DEFAULT '{}'::text[],
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT products_sku_key UNIQUE (sku),
    CONSTRAINT products_price_non_negative CHECK (price_cents >= 0),
    CONSTRAINT products_attributes_object CHECK (jsonb_typeof(attributes) = 'object'),
    CONSTRAINT products_tags_not_null CHECK (array_position(tags, NULL) IS NULL)
);

CREATE TABLE sample.orders (
    id uuid CONSTRAINT orders_pkey PRIMARY KEY,
    user_id uuid NOT NULL,
    status text NOT NULL,
    total_cents bigint NOT NULL,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    tags text[] NOT NULL DEFAULT '{}'::text[],
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT orders_user_fkey
        FOREIGN KEY (user_id) REFERENCES sample.app_users(id)
        ON DELETE RESTRICT,
    CONSTRAINT orders_status_check CHECK (status IN ('open', 'paid', 'canceled', 'refunded')),
    CONSTRAINT orders_total_positive CHECK (total_cents > 0),
    CONSTRAINT orders_metadata_object CHECK (jsonb_typeof(metadata) = 'object'),
    CONSTRAINT orders_tags_not_null CHECK (array_position(tags, NULL) IS NULL)
);

CREATE TABLE sample.order_items (
    id uuid CONSTRAINT order_items_pkey PRIMARY KEY,
    order_id uuid NOT NULL,
    product_id uuid NOT NULL,
    quantity integer NOT NULL,
    unit_price_cents bigint NOT NULL,
    line_total_cents bigint GENERATED ALWAYS AS (quantity::bigint * unit_price_cents) STORED,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    CONSTRAINT order_items_order_fkey
        FOREIGN KEY (order_id) REFERENCES sample.orders(id)
        ON DELETE CASCADE,
    CONSTRAINT order_items_product_fkey
        FOREIGN KEY (product_id) REFERENCES sample.products(id)
        ON DELETE RESTRICT,
    CONSTRAINT order_items_order_product_key UNIQUE (order_id, product_id),
    CONSTRAINT order_items_quantity_positive CHECK (quantity > 0),
    CONSTRAINT order_items_price_non_negative CHECK (unit_price_cents >= 0),
    CONSTRAINT order_items_metadata_object CHECK (jsonb_typeof(metadata) = 'object')
);

CREATE TABLE sample.events (
    id uuid CONSTRAINT events_pkey PRIMARY KEY,
    order_id uuid,
    event_type text NOT NULL,
    payload jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT events_order_fkey
        FOREIGN KEY (order_id) REFERENCES sample.orders(id)
        ON DELETE CASCADE,
    CONSTRAINT events_type_check CHECK (event_type IN ('created', 'paid', 'canceled', 'refunded')),
    CONSTRAINT events_payload_object CHECK (jsonb_typeof(payload) = 'object')
);

CREATE TABLE sample.invoices (
    id uuid CONSTRAINT invoices_pkey PRIMARY KEY,
    invoice_no bigint GENERATED ALWAYS AS IDENTITY,
    customer_id uuid NOT NULL,
    state sample.invoice_state NOT NULL DEFAULT 'draft',
    amount numeric(12, 2) NOT NULL,
    tax_rate numeric(5, 4) NOT NULL DEFAULT 0,
    amount_history numeric[] NOT NULL DEFAULT '{}'::numeric[],
    due_on date NOT NULL,
    issued_at timestamp NOT NULL DEFAULT now(),
    paid_at timestamptz,
    reminder_time time,
    cutoff_time timetz,
    grace_period interval,
    service_days daterange,
    billing_window tstzrange,
    client_ip inet,
    client_network cidr,
    pdf bytea,
    tags text[] NOT NULL DEFAULT '{}'::text[],
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    CONSTRAINT invoices_customer_fkey
        FOREIGN KEY (customer_id) REFERENCES sample.app_users(id)
        ON DELETE RESTRICT,
    CONSTRAINT invoices_invoice_no_key UNIQUE (invoice_no),
    CONSTRAINT invoices_amount_non_negative CHECK (amount >= 0),
    CONSTRAINT invoices_tax_rate_range CHECK (tax_rate >= 0 AND tax_rate <= 1),
    CONSTRAINT invoices_metadata_object CHECK (jsonb_typeof(metadata) = 'object'),
    CONSTRAINT invoices_tags_not_null CHECK (array_position(tags, NULL) IS NULL)
);

CREATE INDEX app_users_organization_status_idx
    ON sample.app_users (organization_id, status);

CREATE INDEX orders_user_created_at_idx
    ON sample.orders (user_id, created_at DESC);

CREATE UNIQUE INDEX orders_one_open_per_user_idx
    ON sample.orders (user_id)
    WHERE status = 'open';

CREATE INDEX orders_metadata_gin_idx
    ON sample.orders USING gin (metadata);

CREATE INDEX orders_tags_gin_idx
    ON sample.orders USING gin (tags);

CREATE VIEW sample.order_search_view AS
SELECT
    o.id,
    o.user_id,
    u.organization_id,
    org.slug AS organization_slug,
    u.email AS user_email,
    o.status,
    o.total_cents,
    o.tags,
    o.metadata,
    o.created_at,
    COALESCE(items.item_count, 0)::bigint AS item_count,
    COALESCE(events.event_count, 0)::bigint AS event_count,
    events.last_event_at
FROM sample.orders o
JOIN sample.app_users u ON u.id = o.user_id
LEFT JOIN sample.organizations org ON org.id = u.organization_id
LEFT JOIN LATERAL (
    SELECT count(*)::bigint AS item_count
    FROM sample.order_items oi
    WHERE oi.order_id = o.id
) items ON true
LEFT JOIN LATERAL (
    SELECT count(*)::bigint AS event_count, max(e.created_at) AS last_event_at
    FROM sample.events e
    WHERE e.order_id = o.id
) events ON true;
