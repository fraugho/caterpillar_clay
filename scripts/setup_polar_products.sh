#!/bin/bash
# Setup script to create Caterpillar Clay products in Polar.sh
# Usage: Add your POLAR_ACCESS_TOKEN to .env and run this script

set -e

# Load environment variables
if [ -f .env ]; then
    export $(grep -v '^#' .env | grep POLAR_ACCESS_TOKEN | xargs)
fi

if [ -z "$POLAR_ACCESS_TOKEN" ] || [ "$POLAR_ACCESS_TOKEN" = "polar_at_..." ]; then
    echo "Error: Please set POLAR_ACCESS_TOKEN in .env with your actual token"
    exit 1
fi

API_URL="https://api.polar.sh/v1"
AUTH_HEADER="Authorization: Bearer $POLAR_ACCESS_TOKEN"

echo "Fetching organization..."
ORG_RESPONSE=$(curl -s "$API_URL/organizations" -H "$AUTH_HEADER")

# Check for error
if echo "$ORG_RESPONSE" | grep -q '"error"'; then
    echo "Error fetching organization: $ORG_RESPONSE"
    exit 1
fi

ORG_ID=$(echo "$ORG_RESPONSE" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)

if [ -z "$ORG_ID" ]; then
    echo "Error: Could not find organization ID"
    echo "Response: $ORG_RESPONSE"
    exit 1
fi

echo "Organization ID: $ORG_ID"
echo ""

# Arrays to store product mappings
declare -A PRODUCT_PRICES

# Function to create a product
create_product() {
    local name="$1"
    local description="$2"
    local price_cents="$3"
    local db_name="$4"

    echo "Creating product: $name ($price_cents cents)..."

    PRODUCT_JSON=$(cat <<EOF
{
    "name": "$name",
    "description": "$description",
    "organization_id": "$ORG_ID",
    "prices": [
        {
            "type": "one_time",
            "amount_type": "fixed",
            "price_amount": $price_cents,
            "price_currency": "usd"
        }
    ]
}
EOF
)

    RESPONSE=$(curl -s -X POST "$API_URL/products" \
        -H "$AUTH_HEADER" \
        -H "Content-Type: application/json" \
        -d "$PRODUCT_JSON")

    if echo "$RESPONSE" | grep -q '"error"'; then
        echo "  Error creating product: $RESPONSE"
        return 1
    fi

    PRODUCT_ID=$(echo "$RESPONSE" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
    PRICE_ID=$(echo "$RESPONSE" | grep -o '"prices":\[{"id":"[^"]*"' | grep -o 'id":"[^"]*"' | head -1 | cut -d'"' -f3)

    echo "  Product ID: $PRODUCT_ID"
    echo "  Price ID: $PRICE_ID"
    echo ""

    # Store the mapping
    PRODUCT_PRICES["$db_name"]="$PRICE_ID"
}

echo "Creating products..."
echo "===================="

# Create the 3 products (db_name should match product names in your database)
create_product \
    "Heart Magnets" \
    "Handcrafted ceramic heart-shaped magnets, perfect for your fridge or magnetic surface. Each piece is unique and made with love." \
    1000 \
    "Heart Magnets"

create_product \
    "Necklace Pendants" \
    "Beautiful handmade ceramic pendants for necklaces. Each pendant is one-of-a-kind with unique glazing and patterns." \
    700 \
    "Necklace Pendants"

create_product \
    "Pins" \
    "Adorable handcrafted ceramic pins to accessorize your bags, jackets, or hats. Each pin is individually made and fired." \
    1000 \
    "Pins"

echo "===================="
echo ""
echo "Products created successfully!"
echo ""
echo "=========================================="
echo "DATABASE UPDATE COMMANDS"
echo "=========================================="
echo ""
echo "Run these commands to link your database products to Polar:"
echo ""

for name in "${!PRODUCT_PRICES[@]}"; do
    price_id="${PRODUCT_PRICES[$name]}"
    echo "-- Update '$name'"
    echo "UPDATE products SET polar_price_id = '$price_id' WHERE name = '$name';"
    echo ""
done

echo ""
echo "To run via turso CLI:"
echo "turso db shell caterpillar-clay"
echo ""
echo "Then paste the UPDATE commands above."
echo ""
echo "=========================================="
echo ""
echo "To list all Polar products and their price IDs later:"
echo "curl -s '$API_URL/products?organization_id=$ORG_ID' -H 'Authorization: Bearer \$POLAR_ACCESS_TOKEN' | jq '.items[] | {name: .name, price_id: .prices[0].id}'"
