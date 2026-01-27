#!/bin/bash
# Test script to verify admin panel authentication is working

BASE_URL="${1:-http://localhost:3000}"
FAILED=0

echo "Testing admin auth at $BASE_URL"
echo "================================"

# Test 1: /gallium should return 401 (unauthorized)
echo -n "Test 1: GET /gallium (should be 401)... "
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/gallium")
if [ "$STATUS" = "401" ]; then
    echo "PASS ($STATUS)"
else
    echo "FAIL (got $STATUS)"
    FAILED=1
fi

# Test 2: /gallium/ should redirect (308) to /gallium
echo -n "Test 2: GET /gallium/ (should be 308 redirect)... "
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/gallium/")
if [ "$STATUS" = "308" ]; then
    echo "PASS ($STATUS)"
else
    echo "FAIL (got $STATUS)"
    FAILED=1
fi

# Test 3: /gallium/ following redirect should be 401
echo -n "Test 3: GET /gallium/ follow redirect (should be 401)... "
STATUS=$(curl -s -L -o /dev/null -w "%{http_code}" "$BASE_URL/gallium/")
if [ "$STATUS" = "401" ]; then
    echo "PASS ($STATUS)"
else
    echo "FAIL (got $STATUS)"
    FAILED=1
fi

# Test 4: /gallium/api/products should return 401
echo -n "Test 4: GET /gallium/api/products (should be 401)... "
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/gallium/api/products")
if [ "$STATUS" = "401" ]; then
    echo "PASS ($STATUS)"
else
    echo "FAIL (got $STATUS)"
    FAILED=1
fi

# Test 5: /gallium/api/dashboard/stats should return 401
echo -n "Test 5: GET /gallium/api/dashboard/stats (should be 401)... "
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/gallium/api/dashboard/stats")
if [ "$STATUS" = "401" ]; then
    echo "PASS ($STATUS)"
else
    echo "FAIL (got $STATUS)"
    FAILED=1
fi

# Test 6: Public storefront should still work (200)
echo -n "Test 6: GET / (public, should be 200)... "
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/")
if [ "$STATUS" = "200" ]; then
    echo "PASS ($STATUS)"
else
    echo "FAIL (got $STATUS)"
    FAILED=1
fi

# Test 7: Public API should still work (200)
echo -n "Test 7: GET /api/products (public, should be 200)... "
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/products")
if [ "$STATUS" = "200" ]; then
    echo "PASS ($STATUS)"
else
    echo "FAIL (got $STATUS)"
    FAILED=1
fi

echo "================================"
if [ "$FAILED" = "0" ]; then
    echo "All tests PASSED!"
    exit 0
else
    echo "Some tests FAILED!"
    exit 1
fi
