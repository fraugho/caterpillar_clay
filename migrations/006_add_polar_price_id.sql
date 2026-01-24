-- Add polar_price_id column to products table for Polar.sh integration
ALTER TABLE products ADD COLUMN polar_price_id TEXT;
