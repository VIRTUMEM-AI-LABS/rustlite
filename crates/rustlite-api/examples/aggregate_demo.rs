/// Example demonstrating aggregate functions and GROUP BY queries in RustLite
use rustlite::{Column, Database, ExecutionContext, Row, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("RustLite Aggregate Functions Demo");
    println!("==================================\n");

    let db = Database::in_memory()?;

    // Example 1: Simple COUNT aggregate
    println!("1. Simple COUNT - Total number of sales:");
    println!("   SQL: SELECT COUNT(*) AS total_sales FROM sales\n");

    let plan = db.prepare("SELECT COUNT(*) AS total_sales FROM sales")?;
    let mut context = ExecutionContext::new();
    context.data.insert(
        "sales".to_string(),
        vec![
            create_sale(1, "Electronics", 1000),
            create_sale(2, "Electronics", 1500),
            create_sale(3, "Books", 200),
            create_sale(4, "Books", 300),
            create_sale(5, "Clothing", 400),
        ],
    );

    let results = db.execute_plan(&plan, context.clone())?;
    print_results(&results);
    println!();

    // Example 2: SUM aggregate
    println!("2. SUM - Total revenue:");
    println!("   SQL: SELECT SUM(amount) AS total_revenue FROM sales\n");

    let plan = db.prepare("SELECT SUM(amount) AS total_revenue FROM sales")?;
    let results = db.execute_plan(&plan, context.clone())?;
    print_results(&results);
    println!();

    // Example 3: AVG aggregate
    println!("3. AVG - Average sale amount:");
    println!("   SQL: SELECT AVG(amount) AS avg_sale FROM sales\n");

    let plan = db.prepare("SELECT AVG(amount) AS avg_sale FROM sales")?;
    let results = db.execute_plan(&plan, context.clone())?;
    print_results(&results);
    println!();

    // Example 4: MIN and MAX aggregates
    println!("4. MIN/MAX - Smallest and largest sales:");
    println!("   SQL: SELECT MIN(amount) AS min_sale, MAX(amount) AS max_sale FROM sales\n");

    let plan = db.prepare("SELECT MIN(amount) AS min_sale, MAX(amount) AS max_sale FROM sales")?;
    let results = db.execute_plan(&plan, context.clone())?;
    print_results(&results);
    println!();

    // Example 5: GROUP BY with SUM
    println!("5. GROUP BY - Total revenue by category:");
    println!("   SQL: SELECT category, SUM(amount) AS revenue FROM sales GROUP BY category\n");

    let plan =
        db.prepare("SELECT category, SUM(amount) AS revenue FROM sales GROUP BY category")?;
    let results = db.execute_plan(&plan, context.clone())?;
    print_results(&results);
    println!();

    // Example 6: GROUP BY with AVG
    println!("6. GROUP BY - Average sale by category:");
    println!("   SQL: SELECT category, AVG(amount) AS avg_sale FROM sales GROUP BY category\n");

    let plan =
        db.prepare("SELECT category, AVG(amount) AS avg_sale FROM sales GROUP BY category")?;
    let results = db.execute_plan(&plan, context.clone())?;
    print_results(&results);
    println!();

    // Example 7: GROUP BY with MIN/MAX
    println!("7. GROUP BY - Min and max sales by category:");
    println!("   SQL: SELECT category, MIN(amount) AS min_sale, MAX(amount) AS max_sale FROM sales GROUP BY category\n");

    let plan = db.prepare("SELECT category, MIN(amount) AS min_sale, MAX(amount) AS max_sale FROM sales GROUP BY category")?;
    let results = db.execute_plan(&plan, context.clone())?;
    print_results(&results);
    println!();

    // Example 8: HAVING clause
    println!("8. HAVING - Filter categories:");
    println!("   SQL: SELECT category, SUM(amount) AS revenue FROM sales GROUP BY category HAVING category = 'Electronics'\n");

    let plan = db.prepare("SELECT category, SUM(amount) AS revenue FROM sales GROUP BY category HAVING category = 'Electronics'")?;
    let results = db.execute_plan(&plan, context)?;
    print_results(&results);
    println!();

    // Use case: Analytics on customer orders
    println!("\nReal-World Use Case: Customer Order Analytics");
    println!("=============================================\n");

    let plan = db.prepare("SELECT customer_id, SUM(amount) AS total_spent, AVG(amount) AS avg_order FROM orders GROUP BY customer_id")?;

    let mut context = ExecutionContext::new();
    context.data.insert(
        "orders".to_string(),
        vec![
            create_order(1001, 150),
            create_order(1001, 200),
            create_order(1001, 350),
            create_order(1002, 500),
            create_order(1002, 600),
            create_order(1003, 100),
        ],
    );

    println!("Customer order statistics:");
    println!("SQL: SELECT customer_id, SUM(amount) AS total_spent, AVG(amount) AS avg_order");
    println!("     FROM orders GROUP BY customer_id\n");

    let results = db.execute_plan(&plan, context)?;
    print_results(&results);

    Ok(())
}

fn create_sale(id: i64, category: &str, amount: i64) -> Row {
    Row {
        columns: vec![
            Column {
                name: "id".to_string(),
                alias: None,
            },
            Column {
                name: "category".to_string(),
                alias: None,
            },
            Column {
                name: "amount".to_string(),
                alias: None,
            },
        ],
        values: vec![
            Value::Integer(id),
            Value::String(category.to_string()),
            Value::Integer(amount),
        ],
    }
}

fn create_order(customer_id: i64, amount: i64) -> Row {
    Row {
        columns: vec![
            Column {
                name: "customer_id".to_string(),
                alias: None,
            },
            Column {
                name: "amount".to_string(),
                alias: None,
            },
        ],
        values: vec![Value::Integer(customer_id), Value::Integer(amount)],
    }
}

fn print_results(results: &[Row]) {
    if results.is_empty() {
        println!("   (no results)");
        return;
    }

    // Print column headers
    print!("   ");
    for col in &results[0].columns {
        print!("{:20}", col.name);
    }
    println!();

    // Print separator
    print!("   ");
    for _ in &results[0].columns {
        print!("{:20}", "--------------------");
    }
    println!();

    // Print rows
    for row in results {
        print!("   ");
        for value in &row.values {
            let display = match value {
                Value::Integer(i) => format!("{}", i),
                Value::Float(f) => format!("{:.2}", f),
                Value::String(s) => s.clone(),
                Value::Boolean(b) => format!("{}", b),
                Value::Null => "NULL".to_string(),
            };
            print!("{:20}", display);
        }
        println!();
    }
}
