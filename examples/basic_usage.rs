use burncloud_database::{Result, create_in_memory_database};

#[tokio::main]
async fn main() -> Result<()> {
    let db = create_in_memory_database().await?;

    println!("In-memory database created successfully!");

    let result = db.execute_query("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)").await?;
    println!("Table created with result: {:?}", result);

    let insert_result = db.execute_query("INSERT INTO users (name, email) VALUES ('Test User', 'test@example.com')").await?;
    println!("Insert result: {:?}", insert_result);

    println!("Database operations completed successfully!");

    db.close().await?;

    Ok(())
}