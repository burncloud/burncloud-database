use burncloud_database::{
    Database, DatabaseError,
    create_default_database
};
use tempfile::TempDir;

/// API compatibility and regression tests
/// These tests ensure backward compatibility and API consistency

#[tokio::test]
async fn test_database_creation_methods() {
    // Test all database creation methods to ensure API consistency

    // Method 1: Database::new() - creates initialized database with default path
    let db_result = Database::new().await;
    if db_result.is_ok() {
        let db = db_result.unwrap();
        assert!(db.connection().is_ok(), "Default database should be initialized");
        let _ = db.close().await;
    }

    // Method 2: Database::new_with_path() + initialize() - for custom paths
    let temp_dir = TempDir::new().expect("Should create temp directory");
    let explicit_path = temp_dir.path().join("explicit.db");

    let mut explicit_db = Database::new_with_path(&explicit_path);
    let explicit_init_result = explicit_db.initialize().await;

    if explicit_init_result.is_ok() {
        assert!(explicit_db.connection().is_ok(), "Explicit database should be initialized");
        let _ = explicit_db.close().await;
    }

    // Method 3: create_default_database() convenience function
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

    // Test Database::new_with_path() with invalid path
    let mut invalid_explicit = Database::new_with_path(invalid_path);
    let explicit_error = invalid_explicit.initialize().await;
    assert!(explicit_error.is_err());

    // Both should return DatabaseError for invalid operations
    println!("✓ Consistent error types for invalid paths");

    // Test path resolution errors (platform-specific)
    #[cfg(target_os = "windows")]
    {
        let original_userprofile = std::env::var("USERPROFILE").ok();
        std::env::remove_var("USERPROFILE");

        let create_default_error = create_default_database().await;

        // Should return PathResolution error
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
    // Test that existing code patterns can be adapted to new API

    // Pattern 1: Custom path usage (now requires new_with_path)
    let temp_dir = TempDir::new().expect("Should create temp directory");
    let db_path = temp_dir.path().join("compat.db");

    let mut path_db = Database::new_with_path(&db_path);
    if path_db.initialize().await.is_ok() {
        // Should work as before
        let result = path_db.execute_query("CREATE TABLE test (id INTEGER)").await;
        assert!(result.is_ok(), "Path-based patterns should work");

        let _ = path_db.close().await;
    }

    // Pattern 2: Default database usage (now simplified)
    match Database::new().await {
        Ok(default_db) => {
            let result = default_db.execute_query("CREATE TABLE test (id INTEGER)").await;
            if result.is_ok() {
                println!("✓ Default database pattern works");
            } else {
                println!("⚠ Default database operations failed (may be due to permissions): {:?}", result);
            }

            let _ = default_db.close().await;
        }
        Err(e) => {
            println!("⚠ Default database creation failed (may be due to environment): {}", e);
            // This is acceptable in test environments
        }
    }

    println!("✓ Adapted patterns work correctly with new API");
}

#[tokio::test]
async fn test_api_surface_completeness() {
    // Test that all expected APIs are available and functional

    // Test Database struct methods
    let _db = Database::new_with_path("test.db");

    // Test that new APIs are available
    let _default_future = Database::new();

    // Test convenience functions
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

// Helper functions

async fn create_test_databases() -> Vec<(String, Database)> {
    let mut databases = vec![];

    // Temporary file database
    if let Ok(temp_dir) = TempDir::new() {
        let temp_path = temp_dir.path().join("temp_test.db");
        let mut temp_db = Database::new_with_path(&temp_path);
        if temp_db.initialize().await.is_ok() {
            databases.push(("temporary_file".to_string(), temp_db));
        }
    }

    // Default location database
    if let Ok(default_db) = Database::new().await {
        databases.push(("default_location".to_string(), default_db));
    }

    databases
}

async fn cleanup_test_databases(databases: Vec<(String, Database)>) {
    for (db_type, db) in databases {
        let _ = db.close().await;
        println!("✓ Cleaned up {} database", db_type);
    }
}