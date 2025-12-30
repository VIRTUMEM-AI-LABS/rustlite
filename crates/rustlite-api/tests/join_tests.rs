use rustlite::{Column, Database, ExecutionContext, Row, Value};

#[test]
fn test_inner_join() {
    let db = Database::in_memory().unwrap();
    let plan = db
        .prepare("SELECT * FROM users INNER JOIN orders ON users.id = orders.user_id")
        .unwrap();

    let mut context = ExecutionContext::new();

    // Users table
    context.data.insert(
        "users".to_string(),
        vec![
            Row {
                columns: vec![
                    Column {
                        name: "id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::Integer(1), Value::String("Alice".to_string())],
            },
            Row {
                columns: vec![
                    Column {
                        name: "id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::Integer(2), Value::String("Bob".to_string())],
            },
        ],
    );

    // Orders table
    context.data.insert(
        "orders".to_string(),
        vec![
            Row {
                columns: vec![
                    Column {
                        name: "order_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "user_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "amount".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::Integer(101), Value::Integer(1), Value::Integer(100)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "order_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "user_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "amount".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::Integer(102), Value::Integer(1), Value::Integer(200)],
            },
        ],
    );

    let results = db.execute_plan(&plan, context).unwrap();

    // Should have 2 rows (Alice has 2 orders, Bob has none)
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].values[1], Value::String("Alice".to_string()));
}

#[test]
fn test_left_join() {
    let db = Database::in_memory().unwrap();
    let plan = db
        .prepare("SELECT * FROM users LEFT JOIN orders ON users.id = orders.user_id")
        .unwrap();

    let mut context = ExecutionContext::new();

    context.data.insert(
        "users".to_string(),
        vec![
            Row {
                columns: vec![
                    Column {
                        name: "id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::Integer(1), Value::String("Alice".to_string())],
            },
            Row {
                columns: vec![
                    Column {
                        name: "id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::Integer(2), Value::String("Bob".to_string())],
            },
        ],
    );

    context.data.insert(
        "orders".to_string(),
        vec![Row {
            columns: vec![
                Column {
                    name: "order_id".to_string(),
                    alias: None,
                },
                Column {
                    name: "user_id".to_string(),
                    alias: None,
                },
            ],
            values: vec![Value::Integer(101), Value::Integer(1)],
        }],
    );

    let results = db.execute_plan(&plan, context).unwrap();

    // Should have 2 rows (Alice with order, Bob with NULL)
    assert_eq!(results.len(), 2);

    // Bob's order fields should be NULL
    let bob_row = results
        .iter()
        .find(|r| r.values[1] == Value::String("Bob".to_string()))
        .unwrap();
    assert_eq!(bob_row.values[2], Value::Null); // order_id should be NULL
}

#[test]
fn test_right_join() {
    let db = Database::in_memory().unwrap();
    let plan = db
        .prepare("SELECT * FROM users RIGHT JOIN orders ON users.id = orders.user_id")
        .unwrap();

    let mut context = ExecutionContext::new();

    context.data.insert(
        "users".to_string(),
        vec![Row {
            columns: vec![
                Column {
                    name: "id".to_string(),
                    alias: None,
                },
                Column {
                    name: "name".to_string(),
                    alias: None,
                },
            ],
            values: vec![Value::Integer(1), Value::String("Alice".to_string())],
        }],
    );

    context.data.insert(
        "orders".to_string(),
        vec![
            Row {
                columns: vec![
                    Column {
                        name: "order_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "user_id".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::Integer(101), Value::Integer(1)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "order_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "user_id".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::Integer(102), Value::Integer(999)], // No matching user
            },
        ],
    );

    let results = db.execute_plan(&plan, context).unwrap();

    // Should have 2 rows (matched + unmatched order)
    assert_eq!(results.len(), 2);

    // Find row with order 102 - should have NULL user fields
    let orphan_order = results
        .iter()
        .find(|r| r.values[2] == Value::Integer(102))
        .unwrap();
    assert_eq!(orphan_order.values[0], Value::Null); // user_id should be NULL
}

#[test]
fn test_full_outer_join() {
    let db = Database::in_memory().unwrap();
    let plan = db
        .prepare("SELECT * FROM users FULL JOIN orders ON users.id = orders.user_id")
        .unwrap();

    let mut context = ExecutionContext::new();

    context.data.insert(
        "users".to_string(),
        vec![
            Row {
                columns: vec![
                    Column {
                        name: "id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::Integer(1), Value::String("Alice".to_string())],
            },
            Row {
                columns: vec![
                    Column {
                        name: "id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::Integer(2), Value::String("Bob".to_string())], // No orders
            },
        ],
    );

    context.data.insert(
        "orders".to_string(),
        vec![
            Row {
                columns: vec![
                    Column {
                        name: "order_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "user_id".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::Integer(101), Value::Integer(1)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "order_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "user_id".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::Integer(102), Value::Integer(999)], // No matching user
            },
        ],
    );

    let results = db.execute_plan(&plan, context).unwrap();

    // Should have 3 rows (Alice+order, Bob+NULL, NULL+orphan order)
    assert_eq!(results.len(), 3);
}

#[test]
fn test_join_with_multiple_matches() {
    let db = Database::in_memory().unwrap();
    let plan = db
        .prepare(
            "SELECT * FROM customers INNER JOIN purchases ON customers.id = purchases.customer_id",
        )
        .unwrap();

    let mut context = ExecutionContext::new();

    context.data.insert(
        "customers".to_string(),
        vec![Row {
            columns: vec![
                Column {
                    name: "id".to_string(),
                    alias: None,
                },
                Column {
                    name: "name".to_string(),
                    alias: None,
                },
            ],
            values: vec![Value::Integer(1), Value::String("Alice".to_string())],
        }],
    );

    context.data.insert(
        "purchases".to_string(),
        vec![
            Row {
                columns: vec![
                    Column {
                        name: "purchase_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "customer_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "item".to_string(),
                        alias: None,
                    },
                ],
                values: vec![
                    Value::Integer(1),
                    Value::Integer(1),
                    Value::String("Laptop".to_string()),
                ],
            },
            Row {
                columns: vec![
                    Column {
                        name: "purchase_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "customer_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "item".to_string(),
                        alias: None,
                    },
                ],
                values: vec![
                    Value::Integer(2),
                    Value::Integer(1),
                    Value::String("Mouse".to_string()),
                ],
            },
            Row {
                columns: vec![
                    Column {
                        name: "purchase_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "customer_id".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "item".to_string(),
                        alias: None,
                    },
                ],
                values: vec![
                    Value::Integer(3),
                    Value::Integer(1),
                    Value::String("Keyboard".to_string()),
                ],
            },
        ],
    );

    let results = db.execute_plan(&plan, context).unwrap();

    // Alice should appear 3 times (once for each purchase)
    assert_eq!(results.len(), 3);
    assert!(results
        .iter()
        .all(|r| r.values[1] == Value::String("Alice".to_string())));
}

#[test]
fn test_join_no_matches() {
    let db = Database::in_memory().unwrap();
    let plan = db
        .prepare("SELECT * FROM users INNER JOIN orders ON users.id = orders.user_id")
        .unwrap();

    let mut context = ExecutionContext::new();

    context.data.insert(
        "users".to_string(),
        vec![Row {
            columns: vec![
                Column {
                    name: "id".to_string(),
                    alias: None,
                },
                Column {
                    name: "name".to_string(),
                    alias: None,
                },
            ],
            values: vec![Value::Integer(1), Value::String("Alice".to_string())],
        }],
    );

    context.data.insert(
        "orders".to_string(),
        vec![Row {
            columns: vec![
                Column {
                    name: "order_id".to_string(),
                    alias: None,
                },
                Column {
                    name: "user_id".to_string(),
                    alias: None,
                },
            ],
            values: vec![Value::Integer(101), Value::Integer(999)], // No matching user
        }],
    );

    let results = db.execute_plan(&plan, context).unwrap();

    // INNER JOIN with no matches should return empty
    assert_eq!(results.len(), 0);
}

#[test]
fn test_join_empty_tables() {
    let db = Database::in_memory().unwrap();
    let plan = db
        .prepare("SELECT * FROM users INNER JOIN orders ON users.id = orders.user_id")
        .unwrap();

    let mut context = ExecutionContext::new();
    context.data.insert("users".to_string(), vec![]);
    context.data.insert("orders".to_string(), vec![]);

    let results = db.execute_plan(&plan, context).unwrap();
    assert_eq!(results.len(), 0);
}
