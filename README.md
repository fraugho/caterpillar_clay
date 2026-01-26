# Caterpillar Clay - Handmade Pottery Shop

A full-stack e-commerce application for a pottery shop built with Rust (Axum) backend and HTMX/Alpine.js frontend.

## Quick Ramp-Up

Recent changes to get you up to speed:

| Feature | Description |
|---------|-------------|
| **Multi-image products** | Products support multiple images with drag-to-reorder. Stored in `product_images` table. |
| **Image carousels** | Storefront shows image carousels on product cards and detail pages. |
| **Stripe auto-sync** | Creating/updating/deleting products or images auto-syncs to Stripe. |
| **Cart persistence** | Cart saves to localStorage with quantities, persists across tabs/refreshes. |
| **SPA routing** | Direct URLs work (e.g., `/product/{id}`, `/cart`, `/orders`). |
| **Unix timestamps** | All `created_ts`/`updated_ts` fields are integers (Unix epoch seconds). |
| **R2 storage** | Product images stored in Cloudflare R2 with public URLs. |
| **Artist page** | `/artist.html` shows artist bio and image, fetched from `site_settings`. |
| **Admin artist settings** | Admin panel ARTIST tab lets you update artist photo and bio. |
| **Centered header** | Logo centered, ARTIST on left, CART/account on right. |
| **Newsletter** | Visitors can subscribe. Admin can send combined "New Products" emails. Uses Resend. |
| **Notify Me** | Out-of-stock products show "Notify Me" button. Customers enter email for one-time restock alert. |
| **Auto restock emails** | When admin restocks a product (0→positive), restock emails auto-send to all subscribers. |
| **Admin batch editing** | Edit multiple products inline, review changes in modal, confirm before saving. |
| **Hidden admin path** | Admin panel at `/gallium/` instead of `/admin/` (security through obscurity + one of Alex's favorite element). |
| **Product styles/variants** | Products can have multiple styles (e.g., "Small Caterpillar", "Be Mine"). Each style has its own stock and optional linked image. |
| **Style image linking** | Styles can link to product images. Selecting a style moves carousel to that image. Images are moved to style folders in R2. |
| **Style-aware notifications** | Customers can subscribe to specific style restocks. Restock emails list which styles are available. |
| **Drag-to-reorder styles** | Admin can reorder styles via drag-and-drop. Visual image picker for linking images to styles. |

### Key Files to Know

| File | Purpose |
|------|---------|
| `static/index.html` | Main storefront SPA (Alpine.js) |
| `static/artist.html` | Artist bio page |
| `static/gallium/index.html` | Admin panel SPA |
| `src/routes/admin/products.rs` | Admin product CRUD + Stripe sync |
| `src/routes/admin/settings.rs` | Admin artist settings API |
| `src/routes/newsletter.rs` | Newsletter subscribe/unsubscribe API |
| `src/routes/admin/newsletter.rs` | Admin newsletter notify endpoints |
| `src/services/resend.rs` | Resend email service for newsletters |
| `src/models/settings.rs` | Site settings model (artist info) |
| `src/models/newsletter.rs` | Newsletter subscriber model |
| `src/models/product_notification.rs` | Product restock notification subscriptions |
| `src/models/product_style.rs` | Product styles/variants model |
| `src/services/stripe.rs` | Stripe API client (payments, products, checkout) |
| `src/models/product.rs` | Product and ProductImage models |
| `src/storage/r2.rs` | Cloudflare R2 storage backend |

## TODO - Cloud Service Setup

| Service | Task | Link/Notes |
|---------|------|------------|
| Google Cloud | Set $1 budget alert | https://console.cloud.google.com/billing/budgets |
| Google Cloud | (Optional) Set up auto-disable | Requires Pub/Sub + Cloud Function |
| Cloudflare | Add rate limiting rule | Dashboard → Security → WAF → Rate Limiting |
| Cloudflare | Enable cache rules for images | Dashboard → Caching → Cache Rules |
| Turso | Nothing needed | Already stops at free tier |
| Cloudflare R2 | Nothing needed | Free tier: 10GB storage, 10M reads/month |

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                   Frontend (HTMX + Alpine.js)                   │
│                         index.html                              │
└─────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Axum Backend                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │   Routes    │  │  Services   │  │   Storage   │             │
│  │  /api/*     │  │  - Clerk    │  │  - Local    │             │
│  │  /admin/*   │  │  - Stripe   │  │  - R2       │             │
│  │  /webhooks  │  │  - EasyPost │  │             │             │
│  └─────────────┘  │  - Email    │  └─────────────┘             │
│                   └─────────────┘                               │
└─────────────────────────────────────────────────────────────────┘
         │                │                    │
         ▼                ▼                    ▼
┌──────────────┐  ┌──────────────┐    ┌──────────────┐
│ libsql/Turso │  │    Clerk     │    │   EasyPost   │
│   Database   │  │    (Auth)    │    │  (Shipping)  │
└──────────────┘  └──────────────┘    └──────────────┘
                         │
         ┌───────────────┴───────────────┐
         ▼                               ▼
┌──────────────┐                ┌──────────────┐
│    Stripe    │                │ Email (SMTP) │
│  (Payments)  │                │   Resend     │
└──────────────┘                └──────────────┘
```

## Tech Stack

- **Backend**: Rust with Axum web framework
- **Database**: libsql/Turso (SQLite compatible, edge-ready)
- **Storage**: Cloudflare R2 (S3-compatible)
- **Authentication**: Clerk
- **Payments**: Stripe (currently using test mode - see Stripe Integration section)
- **Shipping**: EasyPost
- **Email**: SMTP (Resend/SES)
- **Frontend**: HTMX + Alpine.js
- **Styling**: Custom pixel-art CSS

## Project Structure

```
clay/
├── Cargo.toml              # Rust dependencies
├── .env                    # Environment variables (create from .env.example)
├── migrations/             # SQL migrations
│   ├── 001_create_users.sql
│   ├── 002_create_products.sql
│   ├── 003_create_orders.sql
│   ├── 004_create_order_items.sql
│   ├── 005_seed_products.sql
│   ├── 006_add_polar_price_id.sql      # Legacy (renamed in 014)
│   ├── 007_add_polar_product_id.sql    # Legacy (renamed in 014)
│   ├── 008_unix_timestamps.sql
│   ├── 009_product_images.sql
│   ├── 010_site_settings.sql
│   ├── 011_newsletter_subscribers.sql
│   ├── 012_product_notifications.sql
│   ├── 013_product_styles.sql
│   └── 014_rename_polar_to_stripe.sql  # Renames polar_* columns to stripe_*
├── src/
│   ├── main.rs             # Entry point
│   ├── config.rs           # Environment config
│   ├── error.rs            # Error handling
│   ├── db/                 # Database setup
│   ├── models/             # Data models (User, Product, Order)
│   ├── routes/             # API endpoints
│   │   ├── admin/          # Admin panel API
│   │   ├── auth.rs         # Authentication
│   │   ├── products.rs     # Product listing
│   │   ├── cart.rs         # Checkout
│   │   ├── orders.rs       # Order history
│   │   └── webhooks.rs     # Payment/shipping webhooks
│   ├── services/           # External integrations
│   │   ├── clerk.rs        # Clerk auth
│   │   ├── stripe.rs       # Payments & products
│   │   ├── easypost.rs     # Shipping
│   │   └── email.rs        # Notifications
│   ├── storage/            # File storage (Local/R2)
│   └── middleware/         # Auth middleware
├── static/
│   ├── index.html          # Main storefront
│   ├── uploads/            # Product images
│   └── gallium/
│       └── index.html      # Admin panel (hidden path)
└── templates/emails/       # Email templates
```

## Setup

### Prerequisites

- Rust (latest stable)
- External service accounts (optional for development):
  - Clerk (authentication)
  - Stripe (payments) - test keys work for development
  - EasyPost (shipping)
  - Resend/SMTP (email)

### 1. Clone and Configure

```bash
cd clay

# Create environment file
cp .env.example .env
```

Edit `.env` with your configuration:

```bash
# Database (libsql - local SQLite or Turso)
# For local SQLite:
DATABASE_URL=./caterpillar_clay.db
# For Turso (when ready):
# DATABASE_URL=libsql://your-database.turso.io
# TURSO_AUTH_TOKEN=your_auth_token

# For auth (get from clerk.com)
CLERK_SECRET_KEY=sk_test_xxxxx
CLERK_PUBLISHABLE_KEY=pk_test_xxxxx

# For payments (get from stripe.com/dashboard)
# Currently using TEST keys - switch to live keys for production
STRIPE_SECRET_KEY=sk_test_xxxxx
STRIPE_PUBLISHABLE_KEY=pk_test_xxxxx
STRIPE_WEBHOOK_SECRET=whsec_xxxxx

# For shipping (get from easypost.com)
EASYPOST_API_KEY=EZAK_xxxxx
EASYPOST_WEBHOOK_SECRET=whsec_xxxxx

# For email (get from resend.com or use any SMTP)
SMTP_HOST=smtp.resend.com
SMTP_USER=resend
SMTP_PASS=re_xxxxx
FROM_EMAIL=orders@yourdomain.com

# For newsletter (get from resend.com)
RESEND_API_KEY=re_xxxxx

# Storage (local or r2)
STORAGE_TYPE=local
UPLOAD_DIR=./static/uploads

# Cloudflare R2 (when STORAGE_TYPE=r2)
# R2_BUCKET=your-bucket-name
# R2_ACCOUNT_ID=your-cloudflare-account-id
# R2_ACCESS_KEY=your-r2-access-key
# R2_SECRET_KEY=your-r2-secret-key
# R2_PUBLIC_URL=https://pub-xxx.r2.dev

# Server config
BASE_URL=http://localhost:3000
PORT=3000

# Testing (set to true to bypass admin auth)
TESTING_MODE=false
```

### 2. Set Up Database

```bash
# Run all migrations (creates the database file automatically)
for f in migrations/*.sql; do sqlite3 caterpillar_clay.db < "$f"; done
```

**Note:** Timestamps are stored as Unix epoch integers (`i64`) for efficiency. The `created_ts` and `updated_ts` fields use seconds since 1970-01-01.

## Database Schema

### users
| Column | Type | Description |
|--------|------|-------------|
| id | TEXT PK | UUID |
| clerk_id | TEXT UNIQUE | Clerk user ID |
| email | TEXT | User email |
| name | TEXT | Display name |
| is_admin | INTEGER | 1 = admin access |
| created_at | TEXT | ISO timestamp |
| updated_at | TEXT | ISO timestamp |

### products
| Column | Type | Description |
|--------|------|-------------|
| id | TEXT PK | UUID |
| name | TEXT | Product name |
| description | TEXT | Product description |
| price_cents | INTEGER | Price in cents (e.g., 2400 = $24.00) |
| image_path | TEXT | Legacy single image (use product_images instead) |
| stock_quantity | INTEGER | Available stock |
| is_active | INTEGER | 1 = visible in storefront |
| stripe_product_id | TEXT | Stripe product ID (prod_xxx) |
| stripe_price_id | TEXT | Stripe price ID (price_xxx) |
| created_ts | INTEGER | Unix timestamp |
| updated_ts | INTEGER | Unix timestamp |

### product_images
| Column | Type | Description |
|--------|------|-------------|
| id | TEXT PK | UUID |
| product_id | TEXT FK | References products(id) |
| image_path | TEXT | R2 storage path or URL |
| sort_order | INTEGER | Display order (0 = first) |
| created_ts | INTEGER | Unix timestamp |

### orders
| Column | Type | Description |
|--------|------|-------------|
| id | TEXT PK | UUID |
| user_id | TEXT FK | References users(id) |
| status | TEXT | pending/paid/shipped/delivered |
| total_cents | INTEGER | Order total in cents |
| shipping_address | TEXT | JSON address object |
| tracking_number | TEXT | Shipping tracking number |
| easypost_tracker_id | TEXT | EasyPost tracker ID |
| stripe_session_id | TEXT | Stripe checkout session ID |
| created_ts | INTEGER | Unix timestamp |
| updated_ts | INTEGER | Unix timestamp |

### order_items
| Column | Type | Description |
|--------|------|-------------|
| id | TEXT PK | UUID |
| order_id | TEXT FK | References orders(id) |
| product_id | TEXT FK | References products(id) |
| quantity | INTEGER | Item quantity |
| price_cents | INTEGER | Price at time of purchase |

### site_settings
| Column | Type | Description |
|--------|------|-------------|
| key | TEXT PK | Setting key (e.g., `artist_image`, `artist_description`) |
| value | TEXT | Setting value |
| updated_ts | INTEGER | Unix timestamp |

### newsletter_subscribers
| Column | Type | Description |
|--------|------|-------------|
| id | TEXT PK | UUID |
| email | TEXT UNIQUE | Subscriber email |
| subscribed_ts | INTEGER | Unix timestamp |
| unsubscribe_token | TEXT UNIQUE | Token for unsubscribe link |

### product_notifications
| Column | Type | Description |
|--------|------|-------------|
| id | TEXT PK | UUID |
| email | TEXT | Subscriber email |
| product_id | TEXT FK | References products(id) |
| style_id | TEXT FK | References product_styles(id), optional |
| notified | INTEGER | 0 = pending, 1 = sent |
| created_ts | INTEGER | Unix timestamp |
| notified_ts | INTEGER | When notification was sent |

### product_styles
| Column | Type | Description |
|--------|------|-------------|
| id | TEXT PK | UUID |
| product_id | TEXT FK | References products(id) |
| name | TEXT | Style name (e.g., "Small Caterpillar") |
| stock_quantity | INTEGER | Stock for this style |
| image_id | TEXT FK | References product_images(id), optional |
| sort_order | INTEGER | Display order (0 = first) |
| created_ts | INTEGER | Unix timestamp |

### 3. Build and Run

```bash
# Build the project
cargo build

# Run in development mode
cargo run

# Or build for release
cargo build --release
./target/release/caterpillar-clay
```

The server will start on `http://localhost:3000`.

## Usage

### Storefront
- Open `http://localhost:3000` in your browser
- Browse products, add to cart, checkout (requires Clerk auth)

### Admin Panel
- Open `http://localhost:3000/gallium/` (hidden path - gallium is one of Alex's favorite element)
- Requires admin user (set `is_admin = true` in database)
- Or set `TESTING_MODE=true` in `.env` to bypass auth
- Manage products, view orders, add tracking
- **Auto-sync**: Product changes automatically sync to Stripe (create, update, archive, images)

### Making a User Admin

```bash
# Find your user
sqlite3 caterpillar_clay.db "SELECT * FROM users;"

# Make them admin
sqlite3 caterpillar_clay.db "UPDATE users SET is_admin = 1 WHERE email = 'your@email.com';"
```

### Adding Products via Admin API

```bash
# Create a product
curl -X POST http://localhost:3000/admin/api/products \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_CLERK_TOKEN" \
  -d '{
    "name": "Caterpillar Mug",
    "description": "A handcrafted ceramic mug",
    "price_cents": 2400,
    "stock_quantity": 10
  }'
```

## API Endpoints

### Public
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/products` | List active products |
| GET | `/api/products/:id` | Get single product |
| GET | `/api/artist` | Get artist info (image, description) |
| POST | `/api/newsletter/subscribe` | Subscribe to newsletter |
| GET | `/api/newsletter/unsubscribe?token=` | Unsubscribe from newsletter |
| POST | `/api/products/:id/notify` | Subscribe to restock notification |

### Authenticated (Customer)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/orders` | User's order history |
| GET | `/api/orders/:id` | Order details |
| POST | `/api/checkout` | Create checkout session |

### Admin
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/gallium/products` | All products with images |
| POST | `/gallium/products` | Create product (auto-syncs to Stripe) |
| GET | `/gallium/products/:id` | Get single product |
| PUT | `/gallium/products/:id` | Update product (auto-syncs to Stripe) |
| DELETE | `/gallium/products/:id` | Delete product (archives in Stripe) |
| POST | `/gallium/products/:id/images` | Upload images (multipart, auto-syncs) |
| PUT | `/gallium/products/:id/images/reorder` | Reorder images |
| DELETE | `/gallium/products/:id/images/:image_id` | Delete image |
| POST | `/gallium/products/:id/sync-stripe` | Manual Stripe sync |
| POST | `/gallium/products/:id/styles` | Create style |
| PUT | `/gallium/products/:id/styles/:style_id` | Update style |
| DELETE | `/gallium/products/:id/styles/:style_id` | Delete style |
| PUT | `/gallium/products/:id/styles/reorder` | Reorder styles |
| GET | `/gallium/orders` | All orders |
| PUT | `/gallium/orders/:id/status` | Update status |
| POST | `/gallium/orders/:id/tracking` | Add tracking |
| GET | `/gallium/dashboard` | Stats overview |
| GET | `/gallium/settings/artist` | Get artist info |
| PUT | `/gallium/settings/artist` | Update artist description |
| PUT | `/gallium/settings/artist/image` | Upload artist image |
| GET | `/gallium/newsletter/subscribers` | Get subscriber count |
| POST | `/gallium/newsletter/notify/:product_id` | Send new product notification to all subscribers |
| PUT | `/gallium/products-batch` | Batch update multiple products (auto-sends restock emails) |

### Webhooks
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/webhooks/stripe` | Stripe payment confirmations |
| POST | `/webhooks/easypost` | Shipping updates |

## Stripe Integration

The admin dashboard automatically syncs products to Stripe when you create, update, delete, or upload images for products. Each product stores both `stripe_product_id` and `stripe_price_id` for tracking. The admin panel is the single source of truth - all changes propagate to Stripe, R2 storage, and Turso automatically.

### Current Status: TEST MODE

**Important:** The application is currently configured with Stripe **test keys**. Before going to production:

1. Create live API keys at https://dashboard.stripe.com/apikeys
2. Update environment variables:
   ```bash
   STRIPE_SECRET_KEY=sk_live_xxxxx      # Replace sk_test_ with sk_live_
   STRIPE_PUBLISHABLE_KEY=pk_live_xxxxx  # Replace pk_test_ with pk_live_
   ```
3. Set up a production webhook endpoint at https://dashboard.stripe.com/webhooks
4. Update `STRIPE_WEBHOOK_SECRET` with the live webhook signing secret
5. Re-sync all products to create them in the live Stripe account

### Features

- **Product sync**: Products created/updated in admin automatically sync to Stripe
- **Image sync**: Product images (up to 8) are synced as URLs to Stripe products
- **Price management**: Prices are created when products are created. When prices change, a new price is created and the old one is archived (Stripe doesn't allow deleting prices)
- **Checkout sessions**: Stripe Checkout handles the payment flow with shipping address collection
- **Webhook handling**: `checkout.session.completed` events mark orders as paid and decrement stock

### Local Development with Stripe CLI

For testing webhooks locally:

```bash
# Install Stripe CLI
brew install stripe/stripe-cli/stripe

# Login to Stripe
stripe login

# Forward webhooks to your local server
stripe listen --forward-to localhost:3000/webhooks/stripe

# Copy the webhook signing secret (whsec_...) to your .env file
```

### Webhook Events

Configure your webhook endpoint at `https://yourdomain.com/webhooks/stripe` to receive:
- `checkout.session.completed` - Payment successful, order marked as paid

## Deployment

### Build for Production

```bash
cargo build --release
```

### Environment Variables for Production

- Set `BASE_URL` to your production domain
- Configure proper SMTP credentials
- Set up webhook endpoints in Stripe and EasyPost dashboards
- Set `TESTING_MODE=false`

### Running with systemd

Create `/etc/systemd/system/caterpillar-clay.service`:

```ini
[Unit]
Description=Caterpillar Clay Backend
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/path/to/clay
ExecStart=/path/to/clay/target/release/caterpillar-clay
Restart=always
EnvironmentFile=/path/to/clay/.env

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable caterpillar-clay
sudo systemctl start caterpillar-clay
```

## License

MIT
