use burncloud_database::{Result, Database};

#[tokio::main]
async fn main() -> Result<()> {
    // Create a temporary in-memory-like database using a temp path for this example
    let temp_path = std::env::temp_dir().join("basic_usage_example.db");
    let mut db = Database::new_with_path(&temp_path);
    db.initialize().await?;

    println!("Database created successfully!");

    let result = db.execute_query("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)").await?;
    println!("Table created with result: {:?}", result);

    let insert_result = db.execute_query("INSERT INTO users (name, email) VALUES ('Test User', 'test@example.com')").await?;
    println!("Insert result: {:?}", insert_result);

    println!("Database operations completed successfully!");

    db.close().await?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    Ok(())
}