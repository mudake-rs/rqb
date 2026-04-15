CREATE TABLE public.app_users (
    id uuid PRIMARY KEY,
    email text NOT NULL UNIQUE,
    display_name text NOT NULL,
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.orders (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES public.app_users(id),
    status text NOT NULL,
    total_cents bigint NOT NULL CHECK (total_cents > 0),
    created_at timestamptz NOT NULL DEFAULT now()
);
