CREATE TABLE IF NOT EXISTS orders (
    id TEXT PRIMARY KEY,
    user_id TEXT REFERENCES users(id),
    status TEXT DEFAULT 'pending',
    total_cents INTEGER NOT NULL,
    shipping_address TEXT NOT NULL,
    tracking_number TEXT,
    easypost_tracker_id TEXT,
    polar_checkout_id TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_orders_user_id ON orders(user_id);
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_orders_polar_checkout ON orders(polar_checkout_id);
