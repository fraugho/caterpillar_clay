-- Add payment_intent_id to orders for linking refunds
ALTER TABLE orders ADD COLUMN stripe_payment_intent_id TEXT;

-- Create index for faster lookups
CREATE INDEX idx_orders_payment_intent ON orders(stripe_payment_intent_id);
