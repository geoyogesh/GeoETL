use arrow::record_batch::RecordBatch;
use datafusion::prelude::*;
use datafusion_common::Result;
use datafusion_csv::SessionContextCsvExt;

/// Test reading a single CSV file and querying it
#[tokio::test]
async fn test_read_users_csv() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_csv_file("users", "tests/e2e_data/users.csv")
        .await?;

    let df = ctx.sql("SELECT * FROM users WHERE country = 'USA'").await?;
    let batches = df.collect().await?;

    // Should have 2 users from USA (Alice and Diana)
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    assert_eq!(total_rows, 2);

    println!("✓ Successfully read and filtered users.csv");
    Ok(())
}

/// Test reading multiple CSV files
#[tokio::test]
async fn test_register_multiple_tables() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_csv_file("users", "tests/e2e_data/users.csv")
        .await?;
    ctx.register_csv_file("products", "tests/e2e_data/products.csv")
        .await?;
    ctx.register_csv_file("orders", "tests/e2e_data/orders.csv")
        .await?;

    // Verify all tables are registered
    let users = ctx.sql("SELECT COUNT(*) as count FROM users").await?;
    let products = ctx.sql("SELECT COUNT(*) as count FROM products").await?;
    let orders = ctx.sql("SELECT COUNT(*) as count FROM orders").await?;

    let user_count = users.collect().await?;
    let product_count = products.collect().await?;
    let order_count = orders.collect().await?;

    assert_eq!(user_count[0].num_rows(), 1);
    assert_eq!(product_count[0].num_rows(), 1);
    assert_eq!(order_count[0].num_rows(), 1);

    println!("✓ Successfully registered multiple CSV tables");
    Ok(())
}

/// Test joining tables
#[tokio::test]
async fn test_join_users_and_orders() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_csv_file("users", "tests/e2e_data/users.csv")
        .await?;
    ctx.register_csv_file("orders", "tests/e2e_data/orders.csv")
        .await?;

    let df = ctx
        .sql(
            r"
        SELECT
            u.name,
            u.email,
            o.order_id,
            o.total_amount
        FROM users u
        JOIN orders o ON u.user_id = o.user_id
        ORDER BY o.order_id
        ",
        )
        .await?;

    let batches = df.collect().await?;
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();

    // Should have 8 orders
    assert_eq!(total_rows, 8);

    println!("✓ Successfully joined users and orders tables");
    Ok(())
}

/// Test complex query with JOIN, GROUP BY, and aggregation
#[tokio::test]
async fn test_complex_aggregation() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_csv_file("users", "tests/e2e_data/users.csv")
        .await?;
    ctx.register_csv_file("orders", "tests/e2e_data/orders.csv")
        .await?;

    let df = ctx
        .sql(
            r"
        SELECT
            u.name,
            u.country,
            COUNT(o.order_id) as order_count,
            SUM(o.total_amount) as total_spent
        FROM users u
        LEFT JOIN orders o ON u.user_id = o.user_id
        GROUP BY u.name, u.country
        ORDER BY total_spent DESC
        ",
        )
        .await?;

    let batches = df.collect().await?;
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();

    // Should have 5 users
    assert_eq!(total_rows, 5);

    println!("✓ Successfully executed complex aggregation query");
    Ok(())
}

/// Test three-way join
#[tokio::test]
async fn test_three_way_join() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_csv_file("users", "tests/e2e_data/users.csv")
        .await?;
    ctx.register_csv_file("orders", "tests/e2e_data/orders.csv")
        .await?;
    ctx.register_csv_file("products", "tests/e2e_data/products.csv")
        .await?;

    let df = ctx
        .sql(
            r"
        SELECT
            u.name as customer_name,
            p.product_name,
            p.category,
            o.quantity,
            o.total_amount
        FROM orders o
        JOIN users u ON o.user_id = u.user_id
        JOIN products p ON o.product_id = p.product_id
        WHERE p.category = 'Electronics'
        ORDER BY o.order_id
        ",
        )
        .await?;

    let batches = df.collect().await?;
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();

    // Should have orders for electronics products
    assert!(total_rows > 0);

    println!("✓ Successfully executed three-way join");
    Ok(())
}

/// Test reading TSV file with custom delimiter
#[tokio::test]
async fn test_read_tsv_with_delimiter() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_csv_with_delimiter("sales", "tests/e2e_data/sales_data.tsv", b'\t')
        .await?;

    let df = ctx
        .sql(
            r"
        SELECT
            region,
            SUM(revenue) as total_revenue,
            SUM(units_sold) as total_units
        FROM sales
        GROUP BY region
        ORDER BY total_revenue DESC
        ",
        )
        .await?;

    let batches = df.collect().await?;
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();

    // Should have 4 regions
    assert_eq!(total_rows, 4);

    println!("✓ Successfully read and queried TSV file");
    Ok(())
}

/// Test filtering and WHERE clauses
#[tokio::test]
async fn test_filtering_conditions() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_csv_file("products", "tests/e2e_data/products.csv")
        .await?;

    // Test numeric filter
    let df = ctx
        .sql("SELECT product_name, price FROM products WHERE price > 50.0 ORDER BY price")
        .await?;

    let batches = df.collect().await?;
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();

    // Should have products with price > 50
    assert!(total_rows >= 2);

    // Test string filter
    let df2 = ctx
        .sql("SELECT * FROM products WHERE category = 'Electronics'")
        .await?;

    let batches2 = df2.collect().await?;
    let total_rows2: usize = batches2.iter().map(RecordBatch::num_rows).sum();

    assert!(total_rows2 >= 2);

    println!("✓ Successfully tested filtering conditions");
    Ok(())
}

/// Test reading CSV directly into `DataFrame`
#[tokio::test]
async fn test_read_csv_to_dataframe() -> Result<()> {
    let ctx = SessionContext::new();

    let df = ctx.read_csv_file("tests/e2e_data/users.csv").await?;

    // Get count
    let df_count = df.clone().count().await?;
    assert_eq!(df_count, 5);

    // Apply filter
    let filtered = df.filter(col("country").eq(lit("USA")))?;
    let batches = filtered.collect().await?;
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();

    assert_eq!(total_rows, 2);

    println!("✓ Successfully read CSV to DataFrame and filtered");
    Ok(())
}

/// Test aggregations with HAVING clause
#[tokio::test]
async fn test_aggregation_with_having() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_csv_file("orders", "tests/e2e_data/orders.csv")
        .await?;

    let df = ctx
        .sql(
            r"
        SELECT
            user_id,
            COUNT(*) as order_count,
            SUM(total_amount) as total_spent
        FROM orders
        GROUP BY user_id
        HAVING COUNT(*) > 1
        ORDER BY total_spent DESC
        ",
        )
        .await?;

    let batches = df.collect().await?;
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();

    // Should have users with more than 1 order
    assert!(total_rows >= 1);

    println!("✓ Successfully executed aggregation with HAVING clause");
    Ok(())
}

/// Test subqueries
#[tokio::test]
async fn test_subqueries() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_csv_file("users", "tests/e2e_data/users.csv")
        .await?;
    ctx.register_csv_file("orders", "tests/e2e_data/orders.csv")
        .await?;

    let df = ctx
        .sql(
            r"
        SELECT name, email
        FROM users
        WHERE user_id IN (
            SELECT DISTINCT user_id
            FROM orders
            WHERE total_amount > 80.0
        )
        ORDER BY name
        ",
        )
        .await?;

    let batches = df.collect().await?;
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();

    // Should have users with high-value orders
    assert!(total_rows >= 1);

    println!("✓ Successfully executed query with subquery");
    Ok(())
}

/// Full end-to-end test combining all features
#[tokio::test]
async fn test_full_e2e_scenario() -> Result<()> {
    let ctx = SessionContext::new();

    // Register all tables
    ctx.register_csv_file("users", "tests/e2e_data/users.csv")
        .await?;
    ctx.register_csv_file("orders", "tests/e2e_data/orders.csv")
        .await?;
    ctx.register_csv_file("products", "tests/e2e_data/products.csv")
        .await?;
    ctx.register_csv_with_delimiter("sales", "tests/e2e_data/sales_data.tsv", b'\t')
        .await?;

    // Execute a complex analytical query
    let df = ctx
        .sql(
            r"
        SELECT
            u.country,
            p.category,
            COUNT(DISTINCT o.order_id) as num_orders,
            SUM(o.quantity) as total_quantity,
            SUM(o.total_amount) as revenue
        FROM orders o
        JOIN users u ON o.user_id = u.user_id
        JOIN products p ON o.product_id = p.product_id
        GROUP BY u.country, p.category
        ORDER BY revenue DESC
        ",
        )
        .await?;

    let batches = df.collect().await?;
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();

    assert!(total_rows > 0, "Should have aggregated results");

    // Test regional sales data
    let sales_df = ctx
        .sql(
            r"
        SELECT
            region,
            AVG(revenue) as avg_revenue,
            MAX(units_sold) as max_units
        FROM sales
        GROUP BY region
        HAVING AVG(revenue) > 100000
        ",
        )
        .await?;

    let sales_batches = sales_df.collect().await?;
    let sales_rows: usize = sales_batches.iter().map(RecordBatch::num_rows).sum();

    assert!(sales_rows > 0, "Should have high-revenue regions");

    println!("✓ Full end-to-end test passed!");
    println!("  - Registered 4 tables (3 CSV, 1 TSV)");
    println!("  - Executed complex joins and aggregations");
    println!("  - Tested filtering, grouping, and ordering");

    Ok(())
}

/// Test reading CSV from remote object store (R2/HTTP)
#[tokio::test]
async fn test_read_from_remote_object_store() -> Result<()> {
    let ctx = SessionContext::new();

    // Register CSV file from remote R2 bucket
    ctx.register_csv_file(
        "customers",
        "https://pub-f49e62c2a4114dc1bbbb62a1167ab950.r2.dev/readers/csv/customers-100.csv",
    )
    .await?;

    // Test basic query on remote data
    let df = ctx
        .sql("SELECT * FROM customers WHERE \"Country\" = 'Chile'")
        .await?;
    let batches = df.collect().await?;
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();

    // Should have at least 1 customer from Chile
    assert!(total_rows >= 1, "Expected at least 1 customer from Chile");

    println!("✓ Successfully read CSV from remote object store (basic query)");
    Ok(())
}

/// Test filtering on remote object store data
#[tokio::test]
async fn test_filter_remote_object_store() -> Result<()> {
    let ctx = SessionContext::new();

    // Register remote CSV file
    ctx.register_csv_file(
        "customers",
        "https://pub-f49e62c2a4114dc1bbbb62a1167ab950.r2.dev/readers/csv/customers-100.csv",
    )
    .await?;

    // Test filtering with multiple conditions
    let df = ctx
        .sql(
            r#"
        SELECT "First Name", "Last Name", "Company", "Country"
        FROM customers
        WHERE "Country" IN ('Chile', 'Djibouti', 'Bulgaria')
        ORDER BY "Country", "Last Name"
        "#,
        )
        .await?;

    let batches = df.collect().await?;
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();

    // Should have customers from these countries
    assert!(
        total_rows >= 3,
        "Expected at least 3 customers from specified countries"
    );

    println!("✓ Successfully executed filtering on remote object store data");
    Ok(())
}

/// Test aggregation on remote object store data
#[tokio::test]
async fn test_aggregation_remote_object_store() -> Result<()> {
    let ctx = SessionContext::new();

    // Register remote CSV file
    ctx.register_csv_file(
        "customers",
        "https://pub-f49e62c2a4114dc1bbbb62a1167ab950.r2.dev/readers/csv/customers-100.csv",
    )
    .await?;

    // Test aggregation with GROUP BY on remote data
    let df = ctx
        .sql(
            r#"
        SELECT
            "Country",
            COUNT(*) as customer_count
        FROM customers
        GROUP BY "Country"
        HAVING COUNT(*) >= 1
        ORDER BY customer_count DESC
        LIMIT 10
        "#,
        )
        .await?;

    let batches = df.collect().await?;
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();

    // Should have at least 1 country in top 10
    assert!(total_rows >= 1, "Expected at least 1 country");
    assert!(total_rows <= 10, "Should limit to 10 results");

    println!("✓ Successfully executed aggregation on remote object store data");
    Ok(())
}

/// Test column selection on remote object store data
#[tokio::test]
async fn test_column_selection_remote_object_store() -> Result<()> {
    let ctx = SessionContext::new();

    // Register remote CSV file
    ctx.register_csv_file(
        "customers",
        "https://pub-f49e62c2a4114dc1bbbb62a1167ab950.r2.dev/readers/csv/customers-100.csv",
    )
    .await?;

    // Test column projection on remote data
    let df = ctx
        .sql(
            r#"
        SELECT
            "Index",
            "First Name",
            "Last Name",
            "Email",
            "Country"
        FROM customers
        WHERE "Index" <= 10
        ORDER BY "Index"
        "#,
        )
        .await?;

    let batches = df.collect().await?;
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();

    // Should have 10 customers
    assert_eq!(total_rows, 10, "Expected exactly 10 customers");

    // Verify we have the right columns
    assert_eq!(batches[0].num_columns(), 5, "Expected 5 columns");

    println!("✓ Successfully executed column selection on remote object store");
    Ok(())
}

/// Test reading remote CSV into `DataFrame`
#[tokio::test]
async fn test_read_remote_csv_to_dataframe() -> Result<()> {
    let ctx = SessionContext::new();

    // Read remote CSV directly into DataFrame
    let df = ctx
        .read_csv_file(
            "https://pub-f49e62c2a4114dc1bbbb62a1167ab950.r2.dev/readers/csv/customers-100.csv",
        )
        .await?;

    // Get row count
    let df_count = df.clone().count().await?;
    assert_eq!(df_count, 100, "Expected 100 customers");

    // Select specific columns using DataFrame API
    let projected = df.select_columns(&["Index", "First Name", "Last Name", "Email"])?;
    let batches = projected.collect().await?;

    // Verify column projection worked
    assert_eq!(batches[0].num_columns(), 4, "Expected 4 columns");

    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    assert_eq!(total_rows, 100, "Expected all 100 rows");

    println!("✓ Successfully read remote CSV to DataFrame and projected columns");
    Ok(())
}
