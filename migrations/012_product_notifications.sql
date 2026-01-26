-- Product restock notifications (notify me when back in stock)
CREATE TABLE IF NOT EXISTS product_notifications (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    product_id TEXT NOT NULL,
    notified INTEGER NOT NULL DEFAULT 0,
    created_ts INTEGER NOT NULL,
    notified_ts INTEGER,
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE
);

-- Index for finding notifications by product (for sending when restocked)
CREATE INDEX IF NOT EXISTS idx_product_notifications_product ON product_notifications(product_id, notified);

-- Index for checking if email already signed up for a product
CREATE INDEX IF NOT EXISTS idx_product_notifications_email_product ON product_notifications(email, product_id);

-- Prevent duplicate signups for same email + product
CREATE UNIQUE INDEX IF NOT EXISTS idx_product_notifications_unique ON product_notifications(email, product_id) WHERE notified = 0;
