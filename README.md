# Caterpillar Clay - Handmade Pottery Shop

A full-stack e-commerce application for a pottery shop built with Rust (Axum) backend and HTMX/Alpine.js frontend.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                   Frontend (HTMX + Alpine.js)                   │
│                   catepillar_clay.html                          │
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
│  PostgreSQL  │  │    Clerk     │    │   EasyPost   │
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
- **Database**: SQLite with sqlx
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
│   └── 004_create_order_items.sql
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
│   ├── uploads/            # Product images
│   └── admin/
│       └── index.html      # Admin panel
├── templates/emails/       # Email templates
└── catepillar_clay.html    # Main storefront
```

## Setup

### Prerequisites

- Rust (latest stable)
- SQLite 3 (usually pre-installed on most systems)
- External service accounts (optional for development):
  - Clerk (authentication)
  - Polar.sh (payments)
  - EasyPost (shipping)
  - Resend/SMTP (email)

### 1. Clone and Configure

```bash
cd /home/black/code/web/clay

# Create environment file
cp .env.example .env
```

Edit `.env` with your configuration:

```bash
# Database (SQLite - created automatically)
DATABASE_URL=sqlite:./caterpillar_clay.db?mode=rwc

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
```

### 2. Set Up Database

```bash
# Run all migrations (creates the database file automatically)
for f in migrations/*.sql; do sqlite3 caterpillar_clay.db < "$f"; done

# Add some test products
sqlite3 caterpillar_clay.db "INSERT INTO products (id, name, description, price_cents, stock_quantity, is_active) VALUES
('prod-1', 'Caterpillar Mug', 'A cute ceramic mug', 2400, 10, 1),
('prod-2', 'Cocoon Vase', 'Elegant handmade vase', 3600, 5, 1),
('prod-3', 'Leaf Bowl', 'Nature-inspired bowl', 2800, 8, 1);"
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
- Open `http://localhost:3000/catepillar_clay.html` in your browser
- Browse products, add to cart, checkout (requires Clerk auth)

### Admin Panel
- Open `http://localhost:3000/admin/`
- Requires admin user (set `is_admin = true` in database)
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

## Development without External Services

For local development without Clerk/Polar/EasyPost:

1. The app will still run but auth features won't work
2. You can add products directly to the database:

```bash
sqlite3 caterpillar_clay.db "INSERT INTO products (id, name, description, price_cents, stock_quantity, is_active) VALUES
('prod-1', 'Caterpillar Mug', 'A cute ceramic mug', 2400, 10, 1),
('prod-2', 'Cocoon Vase', 'Elegant handmade vase', 3600, 5, 1),
('prod-3', 'Leaf Bowl', 'Nature-inspired bowl', 2800, 8, 1);"
```

3. Products will display on the storefront at `/api/products`

## Deployment

### Build for Production

```bash
cargo build --release
```

### Environment Variables for Production

- Set `BASE_URL` to your production domain
- Configure proper SMTP credentials
- Set up webhook endpoints in Polar.sh and EasyPost dashboards

### Running with systemd

Create `/etc/systemd/system/caterpillar-clay.service`:

```ini
[Unit]
Description=Caterpillar Clay Backend
After=network.target postgresql.service

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
