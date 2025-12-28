//! E-commerce integration tests demonstrating real-world usage patterns
//!
//! This test suite models a complete e-commerce system with:
//! - Product catalog with categories
//! - Shopping carts
//! - Order management
//! - Customer accounts
//! - Inventory tracking
//! - Search and filtering

use rustlite::{Database, IndexType, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Customer {
    id: u64,
    email: String,
    name: String,
    created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Product {
    id: u64,
    sku: String,
    name: String,
    category: String,
    price: u64, // Price in cents
    stock: u32,
    active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct CartItem {
    cart_id: u64,
    product_id: u64,
    quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Order {
    id: u64,
    customer_id: u64,
    total: u64,
    status: String, // pending, confirmed, shipped, delivered, cancelled
    created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct OrderItem {
    order_id: u64,
    product_id: u64,
    quantity: u32,
    price: u64,
}

/// Setup e-commerce database schema with all necessary indexes
fn setup_ecommerce_db() -> Result<Database> {
    let db = Database::in_memory()?;

    // Customer indexes
    db.create_index("customers_pk", IndexType::Hash)?;
    db.create_index("customers_by_email", IndexType::Hash)?;
    db.create_index("customers_by_name", IndexType::BTree)?;

    // Product indexes
    db.create_index("products_pk", IndexType::Hash)?;
    db.create_index("products_by_sku", IndexType::Hash)?;
    db.create_index("products_by_category", IndexType::BTree)?;
    db.create_index("products_active", IndexType::BTree)?;

    // Cart indexes
    db.create_index("cart_items_by_cart", IndexType::BTree)?;
    db.create_index("cart_items_by_product", IndexType::BTree)?;

    // Order indexes
    db.create_index("orders_pk", IndexType::Hash)?;
    db.create_index("orders_by_customer", IndexType::BTree)?;
    db.create_index("orders_by_status", IndexType::BTree)?;
    db.create_index("order_items_by_order", IndexType::BTree)?;

    Ok(db)
}

#[test]
fn test_customer_registration_and_lookup() -> Result<()> {
    let db = setup_ecommerce_db()?;

    let customer = Customer {
        id: 1,
        email: "john@example.com".to_string(),
        name: "John Doe".to_string(),
        created_at: 1640000000,
    };

    // Store customer
    let key = format!("customer:{}", customer.id);
    let value = bincode::serialize(&customer).unwrap();
    db.put(key.as_bytes(), &value)?;

    // Update indexes
    db.index_insert("customers_pk", &customer.id.to_le_bytes(), customer.id)?;
    db.index_insert("customers_by_email", customer.email.as_bytes(), customer.id)?;
    db.index_insert("customers_by_name", customer.name.as_bytes(), customer.id)?;

    // Lookup by email (login scenario)
    let found_ids = db.index_find("customers_by_email", b"john@example.com")?;
    assert_eq!(found_ids, vec![1]);

    // Retrieve customer data
    let key = format!("customer:{}", found_ids[0]);
    let data = db.get(key.as_bytes())?.unwrap();
    let found_customer: Customer = bincode::deserialize(&data).unwrap();
    assert_eq!(found_customer, customer);

    Ok(())
}

#[test]
fn test_product_catalog_management() -> Result<()> {
    let db = setup_ecommerce_db()?;

    let products = vec![
        Product {
            id: 1,
            sku: "LAPTOP-001".to_string(),
            name: "Gaming Laptop".to_string(),
            category: "Electronics".to_string(),
            price: 120000, // $1200.00
            stock: 10,
            active: true,
        },
        Product {
            id: 2,
            sku: "MOUSE-001".to_string(),
            name: "Wireless Mouse".to_string(),
            category: "Electronics".to_string(),
            price: 2500, // $25.00
            stock: 50,
            active: true,
        },
        Product {
            id: 3,
            sku: "DESK-001".to_string(),
            name: "Standing Desk".to_string(),
            category: "Furniture".to_string(),
            price: 35000, // $350.00
            stock: 5,
            active: true,
        },
        Product {
            id: 4,
            sku: "CHAIR-001".to_string(),
            name: "Ergonomic Chair".to_string(),
            category: "Furniture".to_string(),
            price: 25000, // $250.00
            stock: 0,     // Out of stock
            active: false,
        },
    ];

    // Store products
    for product in &products {
        let key = format!("product:{}", product.id);
        let value = bincode::serialize(product).unwrap();
        db.put(key.as_bytes(), &value)?;

        db.index_insert("products_pk", &product.id.to_le_bytes(), product.id)?;
        db.index_insert("products_by_sku", product.sku.as_bytes(), product.id)?;
        db.index_insert(
            "products_by_category",
            product.category.as_bytes(),
            product.id,
        )?;

        if product.active {
            db.index_insert("products_active", &product.id.to_le_bytes(), product.id)?;
        }
    }

    // Find products by category
    let electronics_ids = db.index_find("products_by_category", b"Electronics")?;
    assert_eq!(electronics_ids.len(), 2);

    let furniture_ids = db.index_find("products_by_category", b"Furniture")?;
    assert_eq!(furniture_ids.len(), 2);

    // Find active products only
    let info = db.index_info()?;
    let active_count = info
        .iter()
        .find(|i| i.name == "products_active")
        .map(|i| i.entry_count)
        .unwrap_or(0);
    assert_eq!(active_count, 3); // Exactly 3 active products

    // Lookup by SKU
    let product_ids = db.index_find("products_by_sku", b"LAPTOP-001")?;
    assert_eq!(product_ids, vec![1]);

    Ok(())
}

#[test]
fn test_shopping_cart_operations() -> Result<()> {
    let db = setup_ecommerce_db()?;

    // Setup products first
    let product1 = Product {
        id: 1,
        sku: "PROD-001".to_string(),
        name: "Product 1".to_string(),
        category: "Test".to_string(),
        price: 1000,
        stock: 10,
        active: true,
    };

    let key = format!("product:{}", product1.id);
    db.put(key.as_bytes(), &bincode::serialize(&product1).unwrap())?;
    db.index_insert("products_pk", &product1.id.to_le_bytes(), product1.id)?;

    // Customer's cart
    let cart_id = 100u64;
    let _customer_id = 1u64;

    // Add item to cart
    let cart_item = CartItem {
        cart_id,
        product_id: 1,
        quantity: 2,
    };

    let key = format!("cart:{}:item:{}", cart_id, cart_item.product_id);
    let value = bincode::serialize(&cart_item).unwrap();
    db.put(key.as_bytes(), &value)?;

    db.index_insert(
        "cart_items_by_cart",
        &cart_id.to_le_bytes(),
        cart_item.product_id,
    )?;
    db.index_insert(
        "cart_items_by_product",
        &cart_item.product_id.to_le_bytes(),
        cart_id,
    )?;

    // Retrieve all items in cart
    let item_ids = db.index_find("cart_items_by_cart", &cart_id.to_le_bytes())?;
    assert_eq!(item_ids.len(), 1);

    // Calculate cart total
    let mut total = 0u64;
    for &product_id in &item_ids {
        let cart_key = format!("cart:{}:item:{}", cart_id, product_id);
        let cart_data = db.get(cart_key.as_bytes())?.unwrap();
        let cart_item: CartItem = bincode::deserialize(&cart_data).unwrap();

        let product_key = format!("product:{}", product_id);
        let product_data = db.get(product_key.as_bytes())?.unwrap();
        let product: Product = bincode::deserialize(&product_data).unwrap();

        total += product.price * cart_item.quantity as u64;
    }

    assert_eq!(total, 2000); // 2 items * $10.00

    // Update quantity (increase by 1)
    let mut updated_item = cart_item.clone();
    updated_item.quantity = 3;
    let key = format!("cart:{}:item:{}", cart_id, updated_item.product_id);
    db.put(key.as_bytes(), &bincode::serialize(&updated_item).unwrap())?;

    // Remove item from cart
    let key = format!("cart:{}:item:{}", cart_id, cart_item.product_id);
    db.delete(key.as_bytes())?;
    db.index_remove("cart_items_by_cart", &cart_id.to_le_bytes())?;
    db.index_remove("cart_items_by_product", &cart_item.product_id.to_le_bytes())?;

    let item_ids = db.index_find("cart_items_by_cart", &cart_id.to_le_bytes())?;
    assert_eq!(item_ids.len(), 0);

    Ok(())
}

#[test]
fn test_order_creation_and_fulfillment() -> Result<()> {
    let db = setup_ecommerce_db()?;

    let customer_id = 1u64;
    let order_id = 1000u64;

    // Create order
    let order = Order {
        id: order_id,
        customer_id,
        total: 5000,
        status: "pending".to_string(),
        created_at: 1640000000,
    };

    let key = format!("order:{}", order.id);
    db.put(key.as_bytes(), &bincode::serialize(&order).unwrap())?;

    db.index_insert("orders_pk", &order.id.to_le_bytes(), order.id)?;
    db.index_insert("orders_by_customer", &customer_id.to_le_bytes(), order.id)?;
    db.index_insert("orders_by_status", order.status.as_bytes(), order.id)?;

    // Add order items
    let order_items = [
        OrderItem {
            order_id,
            product_id: 1,
            quantity: 2,
            price: 2500,
        },
        OrderItem {
            order_id,
            product_id: 2,
            quantity: 1,
            price: 2500,
        },
    ];

    for (idx, item) in order_items.iter().enumerate() {
        let key = format!("order:{}:item:{}", order_id, idx);
        db.put(key.as_bytes(), &bincode::serialize(item).unwrap())?;
        db.index_insert("order_items_by_order", &order_id.to_le_bytes(), idx as u64)?;
    }

    // Find all orders for customer
    let customer_orders = db.index_find("orders_by_customer", &customer_id.to_le_bytes())?;
    assert_eq!(customer_orders.len(), 1);

    // Find pending orders
    let pending_orders = db.index_find("orders_by_status", b"pending")?;
    assert_eq!(pending_orders.len(), 1);

    // Update order status (confirmed -> shipped -> delivered)
    let mut updated_order = order.clone();
    updated_order.status = "confirmed".to_string();

    let key = format!("order:{}", order.id);
    db.put(key.as_bytes(), &bincode::serialize(&updated_order).unwrap())?;

    // Remove from pending, add to confirmed
    db.index_remove("orders_by_status", b"pending")?;
    db.index_insert("orders_by_status", b"confirmed", order.id)?;

    let confirmed_orders = db.index_find("orders_by_status", b"confirmed")?;
    assert_eq!(confirmed_orders.len(), 1);

    Ok(())
}

#[test]
fn test_inventory_management() -> Result<()> {
    let db = setup_ecommerce_db()?;

    let mut product = Product {
        id: 1,
        sku: "TEST-001".to_string(),
        name: "Test Product".to_string(),
        category: "Test".to_string(),
        price: 1000,
        stock: 10,
        active: true,
    };

    let key = format!("product:{}", product.id);
    db.put(key.as_bytes(), &bincode::serialize(&product).unwrap())?;
    db.index_insert("products_pk", &product.id.to_le_bytes(), product.id)?;

    // Reduce stock after sale
    product.stock -= 2;
    db.put(key.as_bytes(), &bincode::serialize(&product).unwrap())?;

    // Verify updated stock
    let data = db.get(key.as_bytes())?.unwrap();
    let updated: Product = bincode::deserialize(&data).unwrap();
    assert_eq!(updated.stock, 8);

    // Mark as inactive when out of stock
    product.stock = 0;
    product.active = false;
    db.put(key.as_bytes(), &bincode::serialize(&product).unwrap())?;
    db.index_remove("products_active", &product.id.to_le_bytes())?;

    let data = db.get(key.as_bytes())?.unwrap();
    let updated: Product = bincode::deserialize(&data).unwrap();
    assert!(!updated.active);

    Ok(())
}

#[test]
fn test_customer_order_history() -> Result<()> {
    let db = setup_ecommerce_db()?;

    let customer_id = 1u64;

    // Create multiple orders for customer
    let orders = vec![
        Order {
            id: 1,
            customer_id,
            total: 5000,
            status: "delivered".to_string(),
            created_at: 1640000000,
        },
        Order {
            id: 2,
            customer_id,
            total: 7500,
            status: "delivered".to_string(),
            created_at: 1640100000,
        },
        Order {
            id: 3,
            customer_id,
            total: 3000,
            status: "shipped".to_string(),
            created_at: 1640200000,
        },
    ];

    for order in &orders {
        let key = format!("order:{}", order.id);
        db.put(key.as_bytes(), &bincode::serialize(order).unwrap())?;
        db.index_insert("orders_pk", &order.id.to_le_bytes(), order.id)?;
        db.index_insert("orders_by_customer", &customer_id.to_le_bytes(), order.id)?;
        db.index_insert("orders_by_status", order.status.as_bytes(), order.id)?;
    }

    // Get all orders for customer
    let order_ids = db.index_find("orders_by_customer", &customer_id.to_le_bytes())?;
    assert_eq!(order_ids.len(), 3);

    // Calculate total spent
    let mut total_spent = 0u64;
    for &order_id in &order_ids {
        let key = format!("order:{}", order_id);
        let data = db.get(key.as_bytes())?.unwrap();
        let order: Order = bincode::deserialize(&data).unwrap();
        total_spent += order.total;
    }
    assert_eq!(total_spent, 15500);

    // Get delivered orders only
    let delivered_orders = db.index_find("orders_by_status", b"delivered")?;
    assert_eq!(delivered_orders.len(), 2);

    Ok(())
}

#[test]
fn test_bulk_product_import() -> Result<()> {
    let db = setup_ecommerce_db()?;

    // Simulate bulk import of 100 products
    for i in 1..=100 {
        let product = Product {
            id: i,
            sku: format!("SKU-{:05}", i),
            name: format!("Product {}", i),
            category: if i % 3 == 0 {
                "Electronics".to_string()
            } else if i % 3 == 1 {
                "Furniture".to_string()
            } else {
                "Clothing".to_string()
            },
            price: i * 100,
            stock: (i % 20 + 1) as u32,
            active: i % 10 != 0, // Every 10th product is inactive
        };

        let key = format!("product:{}", product.id);
        db.put(key.as_bytes(), &bincode::serialize(&product).unwrap())?;
        db.index_insert("products_pk", &product.id.to_le_bytes(), product.id)?;
        db.index_insert(
            "products_by_category",
            product.category.as_bytes(),
            product.id,
        )?;

        if product.active {
            db.index_insert("products_active", &product.id.to_le_bytes(), product.id)?;
        }
    }

    // Verify counts by category
    let electronics = db.index_find("products_by_category", b"Electronics")?;
    assert!(electronics.len() >= 30);

    let furniture = db.index_find("products_by_category", b"Furniture")?;
    assert!(furniture.len() >= 30);

    let clothing = db.index_find("products_by_category", b"Clothing")?;
    assert!(clothing.len() >= 30);

    Ok(())
}

#[test]
fn test_order_cancellation_workflow() -> Result<()> {
    let db = setup_ecommerce_db()?;

    let customer_id = 1u64;
    let order_id = 1u64;

    let order = Order {
        id: order_id,
        customer_id,
        total: 10000,
        status: "pending".to_string(),
        created_at: 1640000000,
    };

    let key = format!("order:{}", order.id);
    db.put(key.as_bytes(), &bincode::serialize(&order).unwrap())?;
    db.index_insert("orders_pk", &order.id.to_le_bytes(), order.id)?;
    db.index_insert("orders_by_customer", &customer_id.to_le_bytes(), order.id)?;
    db.index_insert("orders_by_status", b"pending", order.id)?;

    // Cancel order
    let mut cancelled_order = order.clone();
    cancelled_order.status = "cancelled".to_string();

    db.put(
        key.as_bytes(),
        &bincode::serialize(&cancelled_order).unwrap(),
    )?;
    db.index_remove("orders_by_status", b"pending")?;
    db.index_insert("orders_by_status", b"cancelled", order.id)?;

    // Verify cancellation
    let pending = db.index_find("orders_by_status", b"pending")?;
    assert_eq!(pending.len(), 0);

    let cancelled = db.index_find("orders_by_status", b"cancelled")?;
    assert_eq!(cancelled.len(), 1);

    Ok(())
}

#[test]
fn test_concurrent_cart_operations() -> Result<()> {
    let db = setup_ecommerce_db()?;

    // Simulate multiple customers with carts
    let customers = vec![1u64, 2u64, 3u64];
    let product_id = 1u64;

    // Setup product
    let product = Product {
        id: product_id,
        sku: "SHARED-001".to_string(),
        name: "Popular Product".to_string(),
        category: "Test".to_string(),
        price: 1000,
        stock: 100,
        active: true,
    };
    let key = format!("product:{}", product.id);
    db.put(key.as_bytes(), &bincode::serialize(&product).unwrap())?;

    // Multiple customers add same product to cart
    for customer_id in &customers {
        let cart_id = customer_id * 100;
        let cart_item = CartItem {
            cart_id,
            product_id,
            quantity: 2,
        };

        let key = format!("cart:{}:item:{}", cart_id, product_id);
        db.put(key.as_bytes(), &bincode::serialize(&cart_item).unwrap())?;
        db.index_insert("cart_items_by_cart", &cart_id.to_le_bytes(), product_id)?;
        db.index_insert("cart_items_by_product", &product_id.to_le_bytes(), cart_id)?;
    }

    // Find all carts containing this product
    let cart_ids = db.index_find("cart_items_by_product", &product_id.to_le_bytes())?;
    assert_eq!(cart_ids.len(), 3);

    Ok(())
}
