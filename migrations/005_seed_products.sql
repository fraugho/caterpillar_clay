-- Seed initial products
INSERT OR IGNORE INTO products (id, name, description, price_cents, image_path, stock_quantity, is_active, created_at, updated_at)
VALUES
    ('11111111-1111-1111-1111-111111111111', 'Heart Magnets', 'Add a ceramic magnet to your fridge! Each magnet sold separately. Sizes vary from 0.5-1.25" wide.', 1000, '/uploads/heart_magnets.webp', 10, 1, datetime('now'), datetime('now')),
    ('22222222-2222-2222-2222-222222222222', 'Necklace Pendants', 'For people who want to make jewelry at home! Each charm is about 1" wide.', 700, '/uploads/necklace_pendants.jpg', 8, 1, datetime('now'), datetime('now')),
    ('33333333-3333-3333-3333-333333333333', 'Pins', 'Add a ceramic pin to your favorite bag or jacket! Each pin sold separately, and every one has a locking back. Sizes vary between 0.5-1.25" wide.', 1000, '/uploads/pins.webp', 15, 1, datetime('now'), datetime('now'));
