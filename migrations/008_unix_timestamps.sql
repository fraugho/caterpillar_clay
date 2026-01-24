-- Convert TEXT timestamps to INTEGER Unix timestamps (seconds since epoch)
-- SQLite doesn't support ALTER COLUMN, so we add new columns and migrate data

-- Users table
ALTER TABLE users ADD COLUMN created_ts INTEGER;
ALTER TABLE users ADD COLUMN updated_ts INTEGER;
UPDATE users SET
    created_ts = CAST(strftime('%s', created_at) AS INTEGER),
    updated_ts = CAST(strftime('%s', updated_at) AS INTEGER);

-- Products table
ALTER TABLE products ADD COLUMN created_ts INTEGER;
ALTER TABLE products ADD COLUMN updated_ts INTEGER;
UPDATE products SET
    created_ts = CAST(strftime('%s', created_at) AS INTEGER),
    updated_ts = CAST(strftime('%s', updated_at) AS INTEGER);

-- Orders table
ALTER TABLE orders ADD COLUMN created_ts INTEGER;
ALTER TABLE orders ADD COLUMN updated_ts INTEGER;
UPDATE orders SET
    created_ts = CAST(strftime('%s', created_at) AS INTEGER),
    updated_ts = CAST(strftime('%s', updated_at) AS INTEGER);
