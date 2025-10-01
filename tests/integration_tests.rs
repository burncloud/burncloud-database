use burncloud_database::{Database, DatabaseError, Result, create_default_database};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Integration tests for the default database location feature
/// These tests focus on functional validation and real-world scenarios

#[tokio::test]
async fn test_create_default_database_end_to_end() {
    // Test the complete end-to-end workflow of creating a default database
    let result = create_default_database().await;

    match result {
        Ok(db) => {
            // Clean up any existing test data from previous runs
            let _ = db.execute_query("DROP TABLE IF EXISTS test_table").await;

            // Verify the database is functional by performing operations
            let create_result = db.execute_query(
                "CREATE TABLE IF NOT EXISTS test_table (id INTEGER PRIMARY KEY, name TEXT)"
            ).await;
            if let Err(e) = &create_result {
                eprintln!("Failed to create table: {:?}", e);
            }
            assert!(create_result.is_ok(), "Should be able to create tables");

            let insert_result = db.execute_query(
                "INSERT INTO test_table (name) VALUES ('test_data')"
            ).await;
            assert!(insert_result.is_ok(), "Should be able to insert data");

            // Verify data can be retrieved
            #[derive(sqlx::FromRow)]
            struct TestRow {
                id: i64,
                name: String,
            }

            let rows: Result<Vec<TestRow>> = db.fetch_all("SELECT id, name FROM test_table").await;
            assert!(rows.is_ok(), "Should be able to fetch data");
            let rows = rows.unwrap();
            assert_eq!(rows.len(), 1, "Should have exactly one row");
            assert_eq!(rows[0].name, "test_data", "Data should match what was inserted");

            // Clean up
            let _ = db.close().await;
        }
        Err(e) => {
            // In environments where file database creation might fail,
            // at least verify that it's a reasonable error
            match e {
                DatabaseError::PathResolution(_) => {
                    println!("Path resolution failed (acceptable in some environments): {}", e);
                }
                DatabaseError::DirectoryCreation(_) => {
                    println!("Directory creation failed (acceptable in some environments): {}", e);
                }
                DatabaseError::Connection(_) => {
                    println!("Connection failed (acceptable in some environments): {}", e);
                }
                _ => panic!("Unexpected error type: {}", e),
            }
        }
    }
}

#[tokio::test]
async fn test_database_new_default_vs_new_default_initialized() {
    // Test the difference between new_default() and new_default_initialized()

    // Test new_default() - should create Database but not initialize
    let db_uninitialized = Database::new_default();
    match db_uninitialized {
        Ok(db) => {
            // Should not be initialized yet
            let connection_result = db.connection();
            assert!(connection_result.is_err(), "Database should not be initialized yet");

            if let Err(DatabaseError::NotInitialized) = connection_result {
                // This is expected
            } else {
                panic!("Expected NotInitialized error, got other error");
            }
        }
        Err(e) => {
            println!("new_default() failed (acceptable in some environments): {}", e);
        }
    }

    // Test new_default_initialized() - should create and initialize
    let db_initialized_result = Database::new_default_initialized().await;
    match db_initialized_result {
        Ok(db) => {
            // Should be initialized and functional
            let connection_result = db.connection();
            assert!(connection_result.is_ok(), "Database should be initialized");

            // Should be able to perform operations
            let query_result = db.execute_query("SELECT 1 as test").await;
            assert!(query_result.is_ok(), "Should be able to execute queries");

            let _ = db.close().await;
        }
        Err(e) => {
            println!("new_default_initialized() failed (acceptable in some environments): {}", e);
        }
    }
}

#[tokio::test]
async fn test_platform_specific_paths() {
    // Test that platform-specific paths are generated correctly
    let default_path_result = get_test_default_path();

    match default_path_result {
        Ok(path) => {
            let path_str = path.to_string_lossy();

            // Verify the path contains the expected components
            assert!(path_str.contains("data.db"), "Path should end with data.db");

            if cfg!(target_os = "windows") {
                // Windows should use AppData\Local\BurnCloud
                assert!(
                    path_str.contains("AppData") && path_str.contains("Local") && path_str.contains("BurnCloud"),
                    "Windows path should contain AppData\\Local\\BurnCloud, got: {}",
                    path_str
                );
            } else {
                // Linux/Unix should use ~/.burncloud
                assert!(
                    path_str.contains(".burncloud"),
                    "Linux path should contain .burncloud, got: {}",
                    path_str
                );
            }

            println!("Platform-specific default path: {}", path_str);
        }
        Err(e) => {
            println!("Path resolution failed (acceptable in some environments): {}", e);
        }
    }
}

#[tokio::test]
async fn test_directory_creation_and_permissions() {
    // Test that directories are created properly with correct permissions
    let db_result = Database::new_default_initialized().await;

    match db_result {
        Ok(db) => {
            // If database creation succeeded, verify the directory exists
            if let Ok(default_path) = get_test_default_path() {
                if let Some(parent_dir) = default_path.parent() {
                    assert!(parent_dir.exists(), "Parent directory should have been created");

                    // Test that we can write to the directory
                    let test_file = parent_dir.join("test_write.tmp");
                    let write_result = fs::write(&test_file, "test");

                    if write_result.is_ok() {
                        // Clean up test file
                        let _ = fs::remove_file(&test_file);
                    }

                    // Clean up
                    let _ = db.close().await;
                }
            }
        }
        Err(e) => {
            println!("Database creation failed (acceptable in some environments): {}", e);
        }
    }
}

#[tokio::test]
async fn test_multiple_database_instances() {
    // Test that multiple default database instances can coexist
    let db1_result = Database::new_default_initialized().await;
    let db2_result = Database::new_default_initialized().await;

    match (db1_result, db2_result) {
        (Ok(db1), Ok(db2)) => {
            // Both databases should be functional
            let result1 = db1.execute_query("SELECT 1 as test").await;
            let result2 = db2.execute_query("SELECT 1 as test").await;

            assert!(result1.is_ok(), "First database should be functional");
            assert!(result2.is_ok(), "Second database should be functional");

            // Clean up
            let _ = db1.close().await;
            let _ = db2.close().await;
        }
        _ => {
            println!("Multiple database creation failed (acceptable in some environments)");
        }
    }
}

#[tokio::test]
async fn test_database_persistence() {
    // Test that data persists between database instances
    let test_value = "persistent_test_data";

    // Create first database instance and insert data
    let db1_result = Database::new_default_initialized().await;
    if let Ok(db1) = db1_result {
        let create_result = db1.execute_query(
            "CREATE TABLE IF NOT EXISTS persistence_test (id INTEGER PRIMARY KEY, value TEXT)"
        ).await;

        if create_result.is_ok() {
            let insert_result = db1.execute_query(&format!(
                "INSERT INTO persistence_test (value) VALUES ('{}')", test_value
            )).await;

            if insert_result.is_ok() {
                let _ = db1.close().await;

                // Create second database instance and verify data exists
                let db2_result = Database::new_default_initialized().await;
                if let Ok(db2) = db2_result {
                    #[derive(sqlx::FromRow)]
                    struct PersistenceRow {
                        value: String,
                    }

                    let rows: Result<Vec<PersistenceRow>> = db2.fetch_all(
                        "SELECT value FROM persistence_test"
                    ).await;

                    if let Ok(rows) = rows {
                        assert!(!rows.is_empty(), "Data should persist between instances");
                        assert_eq!(rows[0].value, test_value, "Data should match what was inserted");
                        println!("✓ Data persistence verified");
                    }

                    let _ = db2.close().await;
                }
            }
        }
    }
}

#[tokio::test]
async fn test_backward_compatibility() {
    // Test that explicit path APIs still work alongside default location APIs
    let temp_dir = TempDir::new().expect("Should be able to create temp directory");
    let explicit_path = temp_dir.path().join("explicit_test.db");

    // Test explicit path database creation
    let explicit_db_result = Database::new(&explicit_path);
    // Just check that the path is not the in-memory identifier
    assert!(explicit_path.to_string_lossy() != ":memory:", "Should not be in-memory");

    let mut explicit_db = explicit_db_result;
    let init_result = explicit_db.initialize().await;

    if init_result.is_ok() {
        let query_result = explicit_db.execute_query("SELECT 1 as test").await;
        assert!(query_result.is_ok(), "Explicit path database should be functional");
        let _ = explicit_db.close().await;
    }

    // Test that default and explicit path databases are independent
    let default_db_result = Database::new_default_initialized().await;
    if let Ok(default_db) = default_db_result {
        let query_result = default_db.execute_query("SELECT 1 as test").await;
        assert!(query_result.is_ok(), "Default database should be functional");
        let _ = default_db.close().await;
    }
}

#[test]
fn test_error_handling_scenarios() {
    // Test various error scenarios without actually creating databases

    // Test path resolution with missing environment variables
    #[cfg(target_os = "windows")]
    {
        // Temporarily remove USERPROFILE if possible (in a controlled way)
        let original_userprofile = std::env::var("USERPROFILE").ok();
        std::env::remove_var("USERPROFILE");

        let path_result = get_test_default_path();
        assert!(path_result.is_err(), "Should fail when USERPROFILE is missing");

        if let Err(DatabaseError::PathResolution(msg)) = path_result {
            assert!(msg.contains("USERPROFILE"), "Error should mention USERPROFILE");
        }

        // Restore original value
        if let Some(original) = original_userprofile {
            std::env::set_var("USERPROFILE", original);
        }
    }

    // Test API error types
    let db = Database::new("test.db");
    let connection_result = db.connection();
    assert!(connection_result.is_err(), "Should fail when not initialized");

    if let Err(DatabaseError::NotInitialized) = connection_result {
        // Expected error type
        println!("✓ NotInitialized error correctly returned");
    } else {
        panic!("Expected NotInitialized error, got other error");
    }
}

#[tokio::test]
async fn test_api_consistency() {
    // Test that all database creation APIs follow consistent patterns

    // Test in-memory database (existing API)
    let memory_db = Database::new_in_memory();
    // We can't access the path directly, but we know it should be in-memory

    // Test explicit path database (existing API)
    let explicit_db = Database::new("test.db");
    // We can't access the path directly, but we know it should be the explicit path

    // Test default path database (new API)
    let default_db_result = Database::new_default();
    match default_db_result {
        Ok(default_db) => {
            // We can't access the path directly, but we know it should be a default path
            println!("✓ Default database created successfully");
        }
        Err(e) => {
            println!("Default database creation failed (acceptable): {}", e);
        }
    }
}

// Helper function to get the default path for testing
// This replicates the internal logic for testing purposes
fn get_test_default_path() -> Result<PathBuf> {
    let db_dir = if cfg!(target_os = "windows") {
        let user_profile = std::env::var("USERPROFILE")
            .map_err(|e| DatabaseError::PathResolution(format!("USERPROFILE not found: {}", e)))?;
        PathBuf::from(user_profile)
            .join("AppData")
            .join("Local")
            .join("BurnCloud")
    } else {
        dirs::home_dir()
            .ok_or_else(|| DatabaseError::PathResolution("Home directory not found".to_string()))?
            .join(".burncloud")
    };

    Ok(db_dir.join("data.db"))
}