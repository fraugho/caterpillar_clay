-- Product styles/variants table
CREATE TABLE IF NOT EXISTS product_styles (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL,
    name TEXT NOT NULL,
    stock_quantity INTEGER NOT NULL DEFAULT 0,
    image_id TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_ts INTEGER NOT NULL,
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE,
    FOREIGN KEY (image_id) REFERENCES product_images(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_product_styles_product_id ON product_styles(product_id);

-- Add style_id to product_notifications for style-specific subscriptions
ALTER TABLE product_notifications ADD COLUMN style_id TEXT REFERENCES product_styles(id) ON DELETE CASCADE;
