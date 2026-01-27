-- Add weight and dimensions to products for shipping calculations
ALTER TABLE products ADD COLUMN weight_grams INTEGER DEFAULT NULL;
ALTER TABLE products ADD COLUMN length_cm REAL DEFAULT NULL;
ALTER TABLE products ADD COLUMN width_cm REAL DEFAULT NULL;
ALTER TABLE products ADD COLUMN height_cm REAL DEFAULT NULL;
