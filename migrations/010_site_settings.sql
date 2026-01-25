-- Site settings table for configurable content like artist info
CREATE TABLE IF NOT EXISTS site_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_ts INTEGER NOT NULL
);

-- Insert default artist settings
INSERT OR IGNORE INTO site_settings (key, value, updated_ts) VALUES
    ('artist_image', '/artist/Alex.webp', strftime('%s', 'now')),
    ('artist_description', 'I am currently a college student who started on the pottery wheel 10 years ago as a second grader. Since then I have kept with pottery, and I now enjoy making dinnerware and other vessels. I primarily work with blue underglaze or cobalt slip at cone 6, hand painted or slip-trailed. My pieces go through 3 firings to ensure image clarity, and are inspired by flowers I see in the local area!', strftime('%s', 'now'));
