# Caterpillar Clay - Shop Owner Guide

Hey Alex! This guide will help you manage your pottery shop website. Everything you need to do can be done from the admin panel - no coding required.

---

## What Your Website Can Do

Your website is a complete online shop that lets you:

- **Sell your pottery** - Customers can browse, add items to cart, and pay with credit card
- **Manage your products** - Add new pieces, update prices, mark items as sold out
- **Product styles** - Create variations of a product (e.g., "Small Caterpillar", "Large Caterpillar", "Be Mine")
- **Track orders** - See who bought what, their shipping address, and order status
- **Buy shipping labels** - Purchase USPS/UPS/FedEx labels directly from the admin panel
- **Restock notifications** - Customers can sign up to be notified when an out-of-stock item returns
- **Newsletter** - Customers can subscribe to hear about new pieces
- **Share your story** - An "Artist" page with your bio and photo

---

## Accessing the Admin Panel

Your admin panel is at:

**https://caterpillarclay.com/gallium/**

(Named after gallium - one of your favorite elements)

Sign in with your account. Once logged in, you'll see the admin dashboard.

---

## Managing Products

### Adding a New Product

1. Go to the admin panel
2. Click **Add Product**
3. Fill in the details:
   - **Name** - What you call the piece (e.g., "Speckled Blue Mug")
   - **Description** - Tell the story of the piece, materials used, care instructions
   - **Price** - In dollars (e.g., 45.00)
   - **Stock** - How many you have available
   - **Weight** - In grams (needed for shipping rates)
   - **Dimensions** - Length, width, height in inches (for shipping)
   - **Images** - Upload photos. Drag to reorder - the first one becomes the main image
4. Click **Save**

The product will appear on your shop immediately.

### Adding Product Styles/Variations

If you have the same product in different versions (like different sizes or designs):

1. Edit the product
2. Look for the **Styles** section
3. Add styles with their own names and stock quantities (e.g., "Small" with 3 in stock, "Large" with 2 in stock)
4. You can link each style to a specific image in your carousel - when a customer selects that style, it jumps to that image
5. Save

Customers will see style buttons and can pick which one they want.

### Editing a Product

1. Find the product in the admin panel
2. Click **Edit**
3. Make your changes
4. Click **Save**

### Marking Something as Sold Out

1. Edit the product
2. Set the **Stock** to 0 (or set individual style stocks to 0)
3. Save

The item will show "Out of Stock" and customers can click "Notify Me" to get an email when you restock.

### Restocking Items

When you set stock from 0 back to a positive number, anyone who signed up for notifications will automatically get an email letting them know it's available again.

### Deleting a Product

1. Find the product in the admin panel
2. Click **Delete**
3. Confirm when asked

---

## Viewing and Fulfilling Orders

### Checking Orders

1. Go to the **Orders** section in the admin panel
2. Orders are listed with their status
3. Click on an order to see full details:
   - Customer name and email
   - Shipping address
   - Items they ordered (including which style, if applicable)
   - Total paid
   - Shipping cost they paid

### Order Statuses

- **Paid** - Payment received, ready to ship
- **Shipped** - You've sent it out (tracking updates automatically)
- **Delivered** - Carrier confirmed delivery
- **Refunded** - Order was refunded

### Processing Refunds

If you need to refund an order:

1. Open the order
2. Click **Refund**
3. The money goes back to the customer's card automatically

---

## Buying Shipping Labels

You can purchase shipping labels right from the admin panel instead of going to the post office.

### To Buy a Label

1. Open the order you want to ship
2. Click **Buy Label**
3. You'll see shipping options with real-time prices based on:
   - Package weight and dimensions
   - Customer's address
   - Available carriers (USPS, UPS, FedEx)
4. Select the option you want
5. Click **Purchase**
6. The label PDF will be available - print it out

Stick the label on your package and drop it off at the appropriate carrier location (post office for USPS, UPS Store for UPS, etc.).

The tracking number is automatically saved to the order, and the customer can see tracking updates.

### Tips for Shipping

- Make sure product weight and dimensions are filled in for accurate rates
- USPS Priority Mail is usually the best balance of speed and cost for pottery
- The return address on labels uses your shop address from settings

---

## Updating Your Artist Page

Your "Artist" page tells customers your story. To update it:

1. Go to **Settings** in the admin panel
2. Update your **bio** - tell people about yourself, your process, what inspires you
3. Upload or change your **photo**
4. Save

Your changes appear on the website immediately.

---

## Quick Tips

- **Check orders regularly** - Customers appreciate fast shipping
- **Keep stock updated** - Update quantities when you sell at markets or shows
- **Good photos sell** - Natural light, clean backgrounds, show scale with a hand or common object
- **Fill in dimensions** - This makes shipping quotes accurate for customers at checkout
- **Use styles** - If you make the same piece in different glazes or sizes, use styles instead of separate products

---

## Setting Up Your Accounts

To fully transfer ownership of the site, you'll need to create accounts on a few services and share the API keys with me so I can connect them:

- **Stripe** (payments) - stripe.com
- **Shippo** (shipping labels) - goshippo.com
- **Clerk** (user accounts) - clerk.com
- **Resend** (emails) - resend.com

We can walk through this together whenever you're ready. Takes about 30 minutes total.

---

Good luck with the shop.
