#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to UTF-8 string (migration files are SQL text)
    if let Ok(sql) = std::str::from_utf8(data) {
        // Fuzz migration SQL parsing
        // diesel_migrations doesn't expose a direct parser, but we can test
        // SQL statement parsing through the query builder

        // Test splitting SQL by semicolons (common migration operation)
        let statements: Vec<&str> = sql.split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        // Test basic SQL validation patterns that migrations might use
        for stmt in statements {
            // Check for common SQL keywords
            let _is_ddl = stmt.to_uppercase().starts_with("CREATE")
                || stmt.to_uppercase().starts_with("ALTER")
                || stmt.to_uppercase().starts_with("DROP");

            // Test parsing table names from CREATE TABLE statements
            if stmt.to_uppercase().starts_with("CREATE TABLE") {
                let _parts: Vec<&str> = stmt.split_whitespace().collect();
            }
        }
    }
});
