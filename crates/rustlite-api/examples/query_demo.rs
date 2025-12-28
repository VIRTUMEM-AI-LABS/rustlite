/// Query Engine Demo
///
/// Demonstrates SQL-like query capabilities in RustLite v0.4.0+
use rustlite::{Column, Database, ExecutionContext, Row, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== RustLite v0.4.0 Query Engine Demo ===\n");

    // Create an in-memory database
    let db = Database::in_memory()?;

    // Prepare sample data
    let mut context = ExecutionContext::new();

    // Sample users table
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
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "city".to_string(),
                        alias: None,
                    },
                ],
                values: vec![
                    Value::Integer(1),
                    Value::String("Alice".to_string()),
                    Value::Integer(30),
                    Value::String("NYC".to_string()),
                ],
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
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "city".to_string(),
                        alias: None,
                    },
                ],
                values: vec![
                    Value::Integer(2),
                    Value::String("Bob".to_string()),
                    Value::Integer(25),
                    Value::String("SF".to_string()),
                ],
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
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "city".to_string(),
                        alias: None,
                    },
                ],
                values: vec![
                    Value::Integer(3),
                    Value::String("Charlie".to_string()),
                    Value::Integer(35),
                    Value::String("NYC".to_string()),
                ],
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
                    Column {
                        name: "age".to_string(),
                        alias: None,
                    },
                    Column {
                        name: "city".to_string(),
                        alias: None,
                    },
                ],
                values: vec![
                    Value::Integer(4),
                    Value::String("Diana".to_string()),
                    Value::Integer(28),
                    Value::String("LA".to_string()),
                ],
            },
        ],
    );

    // Example 1: Simple SELECT *
    println!("1. SELECT * FROM users");
    let results = db.query("SELECT * FROM users", context.clone())?;
    print_results(&results);

    // Example 2: SELECT with WHERE clause
    println!("\n2. SELECT name, age FROM users WHERE age > 28");
    let results = db.query(
        "SELECT name, age FROM users WHERE age > 28",
        context.clone(),
    )?;
    print_results(&results);

    // Example 3: SELECT with ORDER BY
    println!("\n3. SELECT name, age FROM users ORDER BY age DESC");
    let results = db.query(
        "SELECT name, age FROM users ORDER BY age DESC",
        context.clone(),
    )?;
    print_results(&results);

    // Example 4: SELECT with LIMIT
    println!("\n4. SELECT name FROM users LIMIT 2");
    let results = db.query("SELECT name FROM users LIMIT 2", context.clone())?;
    print_results(&results);

    // Example 5: SELECT with LIMIT and OFFSET
    println!("\n5. SELECT name FROM users LIMIT 2 OFFSET 1");
    let results = db.query("SELECT name FROM users LIMIT 2 OFFSET 1", context.clone())?;
    print_results(&results);

    // Example 6: Complex WHERE with AND
    println!("\n6. SELECT name, city FROM users WHERE age > 25 AND city = 'NYC'");
    let results = db.query(
        "SELECT name, city FROM users WHERE age > 25 AND city = 'NYC'",
        context.clone(),
    )?;
    print_results(&results);

    // Example 7: Prepared statement
    println!("\n7. Using prepared statements");
    let plan = db.prepare("SELECT name, age FROM users WHERE age > 26")?;
    println!("   Query plan: {}", plan);
    let results = db.execute_plan(&plan, context.clone())?;
    print_results(&results);

    // Example 8: COUNT aggregate
    println!("\n8. SELECT COUNT(*) FROM users");
    let results = db.query("SELECT COUNT(*) FROM users", context.clone())?;
    print_results(&results);

    println!("\n=== Query Demo Complete ===");

    Ok(())
}

fn print_results(rows: &[Row]) {
    if rows.is_empty() {
        println!("   No results");
        return;
    }

    // Print header
    print!("   ");
    for col in &rows[0].columns {
        let name = col.alias.as_ref().unwrap_or(&col.name);
        print!("{:15} ", name);
    }
    println!();

    print!("   ");
    for _ in &rows[0].columns {
        print!("{:15} ", "---------------");
    }
    println!();

    // Print rows
    for row in rows {
        print!("   ");
        for val in &row.values {
            match val {
                Value::Integer(i) => print!("{:<15} ", i),
                Value::Float(f) => print!("{:<15.2} ", f),
                Value::String(s) => print!("{:<15} ", s),
                Value::Boolean(b) => print!("{:<15} ", b),
                Value::Null => print!("{:<15} ", "NULL"),
            }
        }
        println!();
    }
}
