-- Rename polar columns to stripe for Stripe integration

-- Rename columns in products table
ALTER TABLE products RENAME COLUMN polar_price_id TO stripe_price_id;
ALTER TABLE products RENAME COLUMN polar_product_id TO stripe_product_id;

-- Rename column in orders table
ALTER TABLE orders RENAME COLUMN polar_checkout_id TO stripe_session_id;

-- Drop old index and create new one
DROP INDEX IF EXISTS idx_orders_polar_checkout;
CREATE INDEX IF NOT EXISTS idx_orders_stripe_session ON orders(stripe_session_id);
