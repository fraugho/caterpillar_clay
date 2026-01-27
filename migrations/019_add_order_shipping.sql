-- Add shipping cost tracking to orders
ALTER TABLE orders ADD COLUMN shipping_cents INTEGER DEFAULT 0;
ALTER TABLE orders ADD COLUMN shipping_carrier TEXT DEFAULT NULL;
ALTER TABLE orders ADD COLUMN shipping_service TEXT DEFAULT NULL;
ALTER TABLE orders ADD COLUMN estimated_delivery_days INTEGER DEFAULT NULL;
