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
│  │  /admin/*   │  │  - Polar    │  │  - S3 (fut) │             │
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
- **Database**: libsql (SQLite compatible, Turso ready)
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
│   └── 005_seed_products.sql
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
│   ├── storage/            # File storage
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
