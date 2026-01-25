-- Newsletter subscribers table
CREATE TABLE IF NOT EXISTS newsletter_subscribers (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    subscribed_ts INTEGER NOT NULL,
    unsubscribe_token TEXT UNIQUE NOT NULL
);

-- Index for quick email lookups
CREATE INDEX IF NOT EXISTS idx_newsletter_email ON newsletter_subscribers(email);
