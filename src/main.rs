use turso::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a local SQLite database connection
    let db = Builder::new_local("eventbook.db").build().await?;
    let conn = db.connect()?;

    // Create a simple table for testing
    conn.execute(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY,
            data TEXT
        )",
        (),
    )
    .await?;

    // Insert a test record
    conn.execute(
        "INSERT INTO events (data) VALUES (?)",
        ("Hello from eventbook!",),
    )
    .await?;

    println!("Database initialized successfully!");

    Ok(())
}
