use rustlite::{Column, Database, ExecutionContext, Row, Value};

#[test]
fn test_simple_select_all() {
    let db = Database::in_memory().unwrap();
    let plan = db.prepare("SELECT * FROM users").unwrap();

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

    let results = db.execute_plan(&plan, context).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_select_with_where_equals() {
    let db = Database::in_memory().unwrap();
    let plan = db.prepare("SELECT * FROM users WHERE age = 30").unwrap();

    let mut context = ExecutionContext::new();
    context.data.insert(
        "users".to_string(),
        vec![
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Alice".to_string()), Value::Integer(30)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Bob".to_string()), Value::Integer(25)],
            },
        ],
    );

    let results = db.execute_plan(&plan, context).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].values[0], Value::String("Alice".to_string()));
}

#[test]
fn test_select_with_where_greater_than() {
    let db = Database::in_memory().unwrap();
    let plan = db.prepare("SELECT * FROM products WHERE price > 50").unwrap();

    let mut context = ExecutionContext::new();
    context.data.insert(
        "products".to_string(),
        vec![
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "price".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Laptop".to_string()), Value::Integer(1000)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "price".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Mouse".to_string()), Value::Integer(25)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "price".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Keyboard".to_string()), Value::Integer(75)],
            },
        ],
    );

    let results = db.execute_plan(&plan, context).unwrap();
    assert_eq!(results.len(), 2); // Laptop and Keyboard
}

#[test]
fn test_select_with_and_condition() {
    let db = Database::in_memory().unwrap();
    let plan = db
        .prepare("SELECT * FROM users WHERE age > 20 AND age < 40")
        .unwrap();

    let mut context = ExecutionContext::new();
    context.data.insert(
        "users".to_string(),
        vec![
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Alice".to_string()), Value::Integer(18)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Bob".to_string()), Value::Integer(25)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Carol".to_string()), Value::Integer(45)],
            },
        ],
    );

    let results = db.execute_plan(&plan, context).unwrap();
    assert_eq!(results.len(), 1); // Only Bob
    assert_eq!(results[0].values[0], Value::String("Bob".to_string()));
}

#[test]
fn test_select_with_or_condition() {
    let db = Database::in_memory().unwrap();
    let plan = db
        .prepare("SELECT * FROM users WHERE age < 20 OR age > 40")
        .unwrap();

    let mut context = ExecutionContext::new();
    context.data.insert(
        "users".to_string(),
        vec![
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Alice".to_string()), Value::Integer(18)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Bob".to_string()), Value::Integer(25)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Carol".to_string()), Value::Integer(45)],
            },
        ],
    );

    let results = db.execute_plan(&plan, context).unwrap();
    assert_eq!(results.len(), 2); // Alice and Carol
}

#[test]
fn test_select_with_limit() {
    let db = Database::in_memory().unwrap();
    let plan = db.prepare("SELECT * FROM users LIMIT 2").unwrap();

    let mut context = ExecutionContext::new();
    context.data.insert(
        "users".to_string(),
        vec![
            Row {
                columns: vec![Column {
                    name: "id".to_string(),
                    alias: None,
                }],
                values: vec![Value::Integer(1)],
            },
            Row {
                columns: vec![Column {
                    name: "id".to_string(),
                    alias: None,
                }],
                values: vec![Value::Integer(2)],
            },
            Row {
                columns: vec![Column {
                    name: "id".to_string(),
                    alias: None,
                }],
                values: vec![Value::Integer(3)],
            },
        ],
    );

    let results = db.execute_plan(&plan, context).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_select_specific_columns() {
    let db = Database::in_memory().unwrap();
    let plan = db.prepare("SELECT name, email FROM users").unwrap();

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
                Column {
                    name: "email".to_string(),
                    alias: None,
                },
            ],
            values: vec![
                Value::Integer(1),
                Value::String("Alice".to_string()),
                Value::String("alice@example.com".to_string()),
            ],
        }],
    );

    let results = db.execute_plan(&plan, context).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].columns.len(), 2);
    assert_eq!(results[0].columns[0].name, "name");
    assert_eq!(results[0].columns[1].name, "email");
}

#[test]
fn test_count_aggregate() {
    let db = Database::in_memory().unwrap();
    let plan = db.prepare("SELECT COUNT(*) FROM users").unwrap();

    let mut context = ExecutionContext::new();
    context.data.insert(
        "users".to_string(),
        vec![
            Row {
                columns: vec![Column {
                    name: "id".to_string(),
                    alias: None,
                }],
                values: vec![Value::Integer(1)],
            },
            Row {
                columns: vec![Column {
                    name: "id".to_string(),
                    alias: None,
                }],
                values: vec![Value::Integer(2)],
            },
            Row {
                columns: vec![Column {
                    name: "id".to_string(),
                    alias: None,
                }],
                values: vec![Value::Integer(3)],
            },
        ],
    );

    let results = db.execute_plan(&plan, context).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].values[0], Value::Integer(3));
}

#[test]
fn test_order_by_ascending() {
    let db = Database::in_memory().unwrap();
    let plan = db.prepare("SELECT * FROM users ORDER BY age ASC").unwrap();

    let mut context = ExecutionContext::new();
    context.data.insert(
        "users".to_string(),
        vec![
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Bob".to_string()), Value::Integer(25)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Alice".to_string()), Value::Integer(18)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Carol".to_string()), Value::Integer(30)],
            },
        ],
    );

    let results = db.execute_plan(&plan, context).unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].values[1], Value::Integer(18)); // Alice first
    assert_eq!(results[1].values[1], Value::Integer(25)); // Bob second
    assert_eq!(results[2].values[1], Value::Integer(30)); // Carol third
}

#[test]
fn test_empty_result_set() {
    let db = Database::in_memory().unwrap();
    let plan = db.prepare("SELECT * FROM users WHERE age > 100").unwrap();

    let mut context = ExecutionContext::new();
    context.data.insert(
        "users".to_string(),
        vec![Row {
            columns: vec![
                Column {
                    name: "name".to_string(),
                    alias: None,
                },
                Column {
                    name: "age".to_string(),
                    alias: None,
                },
            ],
            values: vec![Value::String("Alice".to_string()), Value::Integer(30)],
        }],
    );

    let results = db.execute_plan(&plan, context).unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_complex_query() {
    let db = Database::in_memory().unwrap();
    let plan = db
        .prepare("SELECT name FROM users WHERE age > 20 AND age < 40 ORDER BY age LIMIT 5")
        .unwrap();

    let mut context = ExecutionContext::new();
    context.data.insert(
        "users".to_string(),
        vec![
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Alice".to_string()), Value::Integer(25)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Bob".to_string()), Value::Integer(35)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Carol".to_string()), Value::Integer(18)],
            },
            Row {
                columns: vec![
                    Column {
                        name: "name".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                ],
                values: vec![Value::String("Dave".to_string()), Value::Integer(30)],
            },
        ],
    );

    let results = db.execute_plan(&plan, context).unwrap();
    assert_eq!(results.len(), 3); // Alice, Dave, Bob (Carol filtered out)
    assert_eq!(results[0].values[0], Value::String("Alice".to_string())); // Youngest first
}
