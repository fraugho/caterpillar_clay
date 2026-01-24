# Caterpillar Clay - Handmade Pottery Shop

A full-stack e-commerce application for a pottery shop built with Rust (Axum) backend and HTMX/Alpine.js frontend.

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
│  │  /admin/*   │  │  - Polar    │  │  - R2       │             │
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
│   Polar.sh   │                │ Email (SMTP) │
│  (Payments)  │                │   Resend     │
└──────────────┘                └──────────────┘
```

## Tech Stack

- **Backend**: Rust with Axum web framework
- **Database**: libsql/Turso (SQLite compatible, edge-ready)
- **Storage**: Cloudflare R2 (S3-compatible)
- **Authentication**: Clerk
- **Payments**: Polar.sh
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
│   ├── 006_add_polar_price_id.sql
│   ├── 007_add_polar_product_id.sql
│   └── 008_unix_timestamps.sql
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
│   │   ├── polar.rs        # Payments
│   │   ├── easypost.rs     # Shipping
│   │   └── email.rs        # Notifications
│   ├── storage/            # File storage (Local/R2)
│   └── middleware/         # Auth middleware
├── static/
│   ├── index.html          # Main storefront
│   ├── uploads/            # Product images
│   └── admin/
│       └── index.html      # Admin panel
└── templates/emails/       # Email templates
```

## Setup

### Prerequisites

- Rust (latest stable)
- External service accounts (optional for development):
  - Clerk (authentication)
  - Polar.sh (payments)
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

# For payments (get from polar.sh)
POLAR_ACCESS_TOKEN=polar_at_xxxxx
POLAR_WEBHOOK_SECRET=whsec_xxxxx

# For shipping (get from easypost.com)
EASYPOST_API_KEY=EZAK_xxxxx
EASYPOST_WEBHOOK_SECRET=whsec_xxxxx

# For email (get from resend.com or use any SMTP)
SMTP_HOST=smtp.resend.com
SMTP_USER=resend
SMTP_PASS=re_xxxxx
FROM_EMAIL=orders@yourdomain.com

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
- Open `http://localhost:3000/admin/`
- Requires admin user (set `is_admin = true` in database)
- Or set `TESTING_MODE=true` in `.env` to bypass auth
- Manage products, view orders, add tracking
- **Auto-sync**: Product changes automatically sync to Polar.sh (create, update, archive, images)

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

### Authenticated (Customer)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/orders` | User's order history |
| GET | `/api/orders/:id` | Order details |
| POST | `/api/checkout` | Create checkout session |

### Admin
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/admin/api/dashboard` | Stats overview |
| GET | `/admin/api/products` | All products |
| POST | `/admin/api/products` | Create product |
| PUT | `/admin/api/products/:id` | Update product |
| DELETE | `/admin/api/products/:id` | Delete product |
| POST | `/admin/api/products/:id/image` | Upload image |
| GET | `/admin/api/orders` | All orders |
| PUT | `/admin/api/orders/:id/status` | Update status |
| POST | `/admin/api/orders/:id/tracking` | Add tracking |

### Webhooks
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/webhooks/polar` | Payment confirmations |
| POST | `/api/webhooks/easypost` | Shipping updates |

## Polar.sh Integration

The admin dashboard automatically syncs products to Polar.sh when you create, update, delete, or upload images for products. Each product stores both `polar_product_id` and `polar_price_id` for tracking. The admin panel is the single source of truth - all changes propagate to Polar, R2 storage, and Turso automatically.

### Token Scopes Required

When creating a Polar.sh access token, enable these scopes:
- `products:read`
- `products:write`
- `files:read`
- `files:write`
- `organizations:read`
- `checkouts:read`
- `checkouts:write`

### API Tips

**Trailing slashes are required** - Polar's API returns 307 redirects without them:
```rust
// Wrong - returns 307
format!("{}/v1/products", base_url)

// Correct
format!("{}/v1/products/", base_url)
```

**Organization tokens don't need organization_id** - If using an organization-scoped token, do NOT include `organization_id` in request bodies:
```rust
// Wrong for org tokens - returns "organization_token" error
json!({ "name": "Product", "organization_id": "..." })

// Correct for org tokens
json!({ "name": "Product" })
```

**Image uploads require SHA256 checksums** - When uploading product images:
1. Request an upload URL with the file's SHA256 checksum
2. Include `x-amz-checksum-sha256` header in the S3 PUT request
3. Complete the upload by calling the file complete endpoint

### Webhook Events

Configure your webhook endpoint at `https://yourdomain.com/api/webhooks/polar` to receive:
- `checkout.created` - Customer started checkout
- `checkout.updated` - Checkout status changed
- `order.created` - Order was placed

## Deployment

### Build for Production

```bash
cargo build --release
```

### Environment Variables for Production

- Set `BASE_URL` to your production domain
- Configure proper SMTP credentials
- Set up webhook endpoints in Polar.sh and EasyPost dashboards
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
