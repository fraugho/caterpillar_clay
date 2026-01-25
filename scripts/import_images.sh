#!/usr/bin/env bash

# Import product images from ../utils2 via the admin API
# Usage: ./scripts/import_images.sh [BASE_URL]
#
# Prerequisites:
# - Server must be running
# - TESTING_MODE=true in .env (or be authenticated as admin)

set -e

BASE_URL="${1:-http://localhost:3000}"
UTILS_DIR="../utils2"

echo "Importing images to $BASE_URL"
echo ""

# Function to upload images for a product
upload_product_images() {
    local product_name="$1"
    local product_id="$2"
    local source_dir="$UTILS_DIR/$product_name"

    if [ ! -d "$source_dir" ]; then
        echo "Warning: Source directory not found: $source_dir"
        return
    fi

    echo "Processing: $product_name ($product_id)"

    # Get sorted list of image files (excluding .DS_Store)
    for img in $(ls "$source_dir" | grep -v '.DS_Store' | sort -V); do
        src_path="$source_dir/$img"

        if [ -f "$src_path" ]; then
            echo "  Uploading: $img"

            # Upload via admin API
            response=$(curl -s -X POST "$BASE_URL/admin/api/products/$product_id/images" \
                -F "file=@$src_path" \
                -w "\n%{http_code}")

            http_code=$(echo "$response" | tail -1)

            if [ "$http_code" = "200" ]; then
                echo "    ✓ Success"
            else
                echo "    ✗ Failed (HTTP $http_code)"
            fi
        fi
    done

    echo ""
}

# Upload for each product
upload_product_images "Heart Magnets" "11111111-1111-1111-1111-111111111111"
upload_product_images "Necklace Pendants" "22222222-2222-2222-2222-222222222222"
upload_product_images "Pins" "33333333-3333-3333-3333-333333333333"

echo "Done!"
