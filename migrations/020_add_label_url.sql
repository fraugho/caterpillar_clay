-- Add label URL to orders for purchased shipping labels
ALTER TABLE orders ADD COLUMN label_url TEXT DEFAULT NULL;
