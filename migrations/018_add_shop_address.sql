-- Add shop address settings for shipping origin
INSERT OR IGNORE INTO site_settings (key, value, updated_ts) VALUES
    ('shop_name', 'Caterpillar Clay', strftime('%s', 'now')),
    ('shop_street1', '', strftime('%s', 'now')),
    ('shop_street2', '', strftime('%s', 'now')),
    ('shop_city', '', strftime('%s', 'now')),
    ('shop_state', '', strftime('%s', 'now')),
    ('shop_zip', '', strftime('%s', 'now')),
    ('shop_country', 'US', strftime('%s', 'now')),
    ('shop_phone', '', strftime('%s', 'now')),
    ('shipping_unit_system', 'metric', strftime('%s', 'now'));
