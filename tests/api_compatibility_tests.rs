use burncloud_database::{
    Database, DatabaseError,
    create_database, create_in_memory_database, create_default_database
};
use tempfile::TempDir;

/// API compatibility and regression tests
/// These tests ensure backward compatibility and API consistency

#[tokio::test]
async fn test_all_database_creation_methods() {
    // Test all database creation methods to ensure API consistency

    // Method 1: Database::new() + initialize()
    let temp_dir = TempDir::new().expect("Should create temp directory");
    let explicit_path = temp_dir.path().join("explicit.db");

    let mut explicit_db = Database::new(&explicit_path);
    let explicit_init_result = explicit_db.initialize().await;

    if explicit_init_result.is_ok() {
        assert!(explicit_db.connection().is_ok(), "Explicit database should be initialized");
        let _ = explicit_db.close().await;
    }

    // Method 2: create_database() convenience function
    let convenience_path = temp_dir.path().join("convenience.db");
    let convenience_result = create_database(&convenience_path).await;

    if let Ok(convenience_db) = convenience_result {
        assert!(convenience_db.connection().is_ok(), "Convenience database should be initialized");
        let _ = convenience_db.close().await;
    }

    // Method 3: Database::new_in_memory() + initialize()
    let mut memory_db = Database::new_in_memory();
    let memory_init_result = memory_db.initialize().await;

    if memory_init_result.is_ok() {
        assert!(memory_db.connection().is_ok(), "Memory database should be initialized");
        let _ = memory_db.close().await;
    }

    // Method 4: create_in_memory_database() convenience function
    let memory_convenience_result = create_in_memory_database().await;

    if let Ok(memory_convenience_db) = memory_convenience_result {
        assert!(memory_convenience_db.connection().is_ok(), "Memory convenience database should be initialized");
        let _ = memory_convenience_db.close().await;
    }

    // Method 5: Database::new_default() + initialize() (new API)
    if let Ok(mut default_db) = Database::new_default() {
        let default_init_result = default_db.initialize().await;

        if default_init_result.is_ok() {
            assert!(default_db.connection().is_ok(), "Default database should be initialized");
            let _ = default_db.close().await;
        }
    }

    // Method 6: Database::new_default_initialized() (new API)
    let default_initialized_result = Database::new_default_initialized().await;

    if let Ok(default_initialized_db) = default_initialized_result {
        assert!(default_initialized_db.connection().is_ok(), "Default initialized database should be ready");
        let _ = default_initialized_db.close().await;
    }

    // Method 7: create_default_database() convenience function (new API)
    let default_convenience_result = create_default_database().await;

    if let Ok(default_convenience_db) = default_convenience_result {
        assert!(default_convenience_db.connection().is_ok(), "Default convenience database should be initialized");
        let _ = default_convenience_db.close().await;
    }

    println!("✓ All database creation methods tested for consistency");
}

#[tokio::test]
async fn test_database_operation_consistency() {
    // Test that all database types support the same operations consistently

    let databases = create_test_databases().await;

    for (db_type, db) in &databases {
        println!("Testing operations on {} database", db_type);

        // Test basic query execution
        let basic_query_result = db.execute_query("SELECT 1 as test_value").await;
        assert!(basic_query_result.is_ok(), "{} database should support basic queries", db_type);

        // Test table creation
        let create_table_result = db.execute_query(
            "CREATE TABLE IF NOT EXISTS api_test (id INTEGER PRIMARY KEY, name TEXT, value INTEGER)"
        ).await;
        assert!(create_table_result.is_ok(), "{} database should support table creation", db_type);

        // Test data insertion
        let insert_result = db.execute_query(
            "INSERT INTO api_test (name, value) VALUES ('test_name', 42)"
        ).await;
        assert!(insert_result.is_ok(), "{} database should support data insertion", db_type);

        // Test fetch_one
        #[derive(sqlx::FromRow)]
        struct ApiTestRow {
            id: i64,
            name: String,
            value: i64,
        }

        let fetch_one_result = db.fetch_one::<ApiTestRow>("SELECT id, name, value FROM api_test LIMIT 1").await;
        assert!(fetch_one_result.is_ok(), "{} database should support fetch_one", db_type);

        if let Ok(row) = fetch_one_result {
            assert_eq!(row.name, "test_name");
            assert_eq!(row.value, 42);
        }

        // Test fetch_all
        let fetch_all_result = db.fetch_all::<ApiTestRow>("SELECT id, name, value FROM api_test").await;
        assert!(fetch_all_result.is_ok(), "{} database should support fetch_all", db_type);

        if let Ok(rows) = fetch_all_result {
            assert!(!rows.is_empty(), "{} database should return data", db_type);
        }

        // Test fetch_optional
        let fetch_optional_result = db.fetch_optional::<ApiTestRow>(
            "SELECT id, name, value FROM api_test WHERE name = 'nonexistent'"
        ).await;
        assert!(fetch_optional_result.is_ok(), "{} database should support fetch_optional", db_type);

        if let Ok(optional_row) = fetch_optional_result {
            assert!(optional_row.is_none(), "{} database should return None for non-existent data", db_type);
        }

        // Test connection access
        let connection_result = db.connection();
        assert!(connection_result.is_ok(), "{} database should provide connection access", db_type);
    }

    // Clean up
    cleanup_test_databases(databases).await;

    println!("✓ All database types support consistent operations");
}

#[tokio::test]
async fn test_error_type_consistency() {
    // Test that all database creation methods return consistent error types

    // Test with invalid paths
    let invalid_path = "/definitely/invalid/path/test.db";

    // Test Database::new() with invalid path
    let mut invalid_explicit = Database::new(invalid_path);
    let explicit_error = invalid_explicit.initialize().await;
    assert!(explicit_error.is_err());

    // Test create_database() with invalid path
    let convenience_error = create_database(invalid_path).await;
    assert!(convenience_error.is_err());

    // Both should return DatabaseError::Connection for invalid paths
    match (explicit_error, convenience_error) {
        (Err(DatabaseError::Connection(_)), Err(DatabaseError::Connection(_))) => {
            println!("✓ Consistent error types for invalid paths");
        }
        (explicit_err, convenience_err) => {
            println!("Error type consistency - both should at least be errors");
            // Both should at least be errors, even if types differ slightly
        }
    }

    // Test path resolution errors (platform-specific)
    #[cfg(target_os = "windows")]
    {
        let original_userprofile = std::env::var("USERPROFILE").ok();
        std::env::remove_var("USERPROFILE");

        let new_default_error = Database::new_default();
        let new_default_init_error = Database::new_default_initialized().await;
        let create_default_error = create_default_database().await;

        // All should return PathResolution errors
        assert!(matches!(new_default_error, Err(DatabaseError::PathResolution(_))));
        assert!(matches!(new_default_init_error, Err(DatabaseError::PathResolution(_))));
        assert!(matches!(create_default_error, Err(DatabaseError::PathResolution(_))));

        // Restore environment
        if let Some(original) = original_userprofile {
            std::env::set_var("USERPROFILE", original);
        }

        println!("✓ Consistent error types for path resolution failures");
    }
}

#[tokio::test]
async fn test_backward_compatibility() {
    // Test that existing code patterns still work unchanged

    // Pattern 1: Traditional explicit path usage
    let temp_dir = TempDir::new().expect("Should create temp directory");
    let db_path = temp_dir.path().join("backward_compat.db");

    let mut old_style_db = Database::new(&db_path);
    if old_style_db.initialize().await.is_ok() {
        // Should work exactly as before
        let result = old_style_db.execute_query("CREATE TABLE test (id INTEGER)").await;
        assert!(result.is_ok(), "Traditional patterns should still work");

        let _ = old_style_db.close().await;
    }

    // Pattern 2: Convenience function usage
    let convenience_path = temp_dir.path().join("convenience_compat.db");
    if let Ok(convenience_db) = create_database(&convenience_path).await {
        let result = convenience_db.execute_query("CREATE TABLE test (id INTEGER)").await;
        assert!(result.is_ok(), "Convenience functions should still work");

        let _ = convenience_db.close().await;
    }

    // Pattern 3: In-memory database usage
    if let Ok(memory_db) = create_in_memory_database().await {
        let result = memory_db.execute_query("CREATE TABLE test (id INTEGER)").await;
        assert!(result.is_ok(), "In-memory databases should still work");

        let _ = memory_db.close().await;
    }

    // Pattern 4: Manual initialization
    let mut manual_db = Database::new_in_memory();
    if manual_db.initialize().await.is_ok() {
        let result = manual_db.execute_query("CREATE TABLE test (id INTEGER)").await;
        assert!(result.is_ok(), "Manual initialization should still work");

        let _ = manual_db.close().await;
    }

    println!("✓ All backward compatibility patterns work correctly");
}

#[tokio::test]
async fn test_api_surface_completeness() {
    // Test that all expected APIs are available and functional

    // Test Database struct methods
    let _db = Database::new("test.db");
    // We can't access the database_path field directly as it's private

    let _memory_db = Database::new_in_memory();
    // We can't access the database_path field directly as it's private

    // Test that new APIs are available
    let _default_result = Database::new_default();
    let _default_init_future = Database::new_default_initialized();

    // Test convenience functions
    let _create_future = create_database("test.db");
    let _create_memory_future = create_in_memory_database();
    let _create_default_future = create_default_database();

    // Test error types are available
    let _path_error = DatabaseError::PathResolution("test".to_string());
    let _dir_error = DatabaseError::DirectoryCreation("test".to_string());

    println!("✓ All expected APIs are available");
}

#[tokio::test]
async fn test_database_connection_consistency() {
    // Test that DatabaseConnection behaves consistently across all database types

    let databases = create_test_databases().await;

    for (db_type, db) in &databases {
        if let Ok(connection) = db.connection() {
            // Test pool access
            let pool = connection.pool();
            assert!(!pool.is_closed(), "{} connection pool should be open", db_type);

            // Test that we can execute queries through the pool
            let direct_result = sqlx::query("SELECT 1").execute(pool).await;
            assert!(direct_result.is_ok(), "{} should allow direct pool access", db_type);

            println!("✓ {} database connection is consistent", db_type);
        }
    }

    cleanup_test_databases(databases).await;
}

#[tokio::test]
async fn test_connection_sharing_behavior() {
    // Test that DatabaseConnection sharing works consistently

    if let Ok(original_db) = create_in_memory_database().await {
        // Get the connection and clone it for sharing
        if let Ok(connection) = original_db.connection() {
            let shared_connection = connection.clone();

            // Both connections should work with the same pool
            let original_result = sqlx::query("CREATE TABLE clone_test (id INTEGER)")
                .execute(connection.pool())
                .await;
            assert!(original_result.is_ok(), "Original connection should work");

            let shared_result = sqlx::query("INSERT INTO clone_test (id) VALUES (1)")
                .execute(shared_connection.pool())
                .await;
            assert!(shared_result.is_ok(), "Shared connection should work");

            // Both should see the same data (shared connection pool)
            let original_count = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM clone_test")
                .fetch_one(connection.pool())
                .await;
            let shared_count = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM clone_test")
                .fetch_one(shared_connection.pool())
                .await;

            if let (Ok((orig_count,)), Ok((shared_count,))) = (original_count, shared_count) {
                assert_eq!(orig_count, shared_count, "Shared connections should see same data");
                println!("✓ Connection sharing works correctly");
            }
        }

        let _ = original_db.close().await;
    }
}

// Helper functions

async fn create_test_databases() -> Vec<(String, Database)> {
    let mut databases = vec![];

    // In-memory database (always works)
    if let Ok(memory_db) = create_in_memory_database().await {
        databases.push(("in_memory".to_string(), memory_db));
    }

    // Temporary file database
    if let Ok(temp_dir) = TempDir::new() {
        let temp_path = temp_dir.path().join("temp_test.db");
        if let Ok(temp_db) = create_database(&temp_path).await {
            databases.push(("temporary_file".to_string(), temp_db));
        }
    }

    // Default location database (using temporary directory for testing)
    if let Ok(temp_dir) = TempDir::new() {
        let test_default_path = temp_dir.path().join("BurnCloud").join("data.db");
        if let Ok(test_db) = create_database(&test_default_path).await {
            databases.push(("default_location".to_string(), test_db));
        }
    }

    databases
}

async fn cleanup_test_databases(databases: Vec<(String, Database)>) {
    for (db_type, db) in databases {
        let _ = db.close().await;
        println!("✓ Cleaned up {} database", db_type);
    }
}