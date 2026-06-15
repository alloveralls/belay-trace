use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};

use crate::error::BelayError;

pub const LATEST_SCHEMA_VERSION: i64 = 2;

const MIGRATION_1: &str = r#"
CREATE TABLE entries (
    id INTEGER PRIMARY KEY,
    display_id TEXT NOT NULL,
    type TEXT NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 1),
    body TEXT NOT NULL DEFAULT '',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    source_path TEXT,
    content_hash TEXT NOT NULL DEFAULT ''
);

CREATE UNIQUE INDEX idx_entries_display_id ON entries(display_id);
CREATE INDEX idx_entries_type_status ON entries(type, status);

CREATE TABLE entry_tags (
    entry_id INTEGER NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (entry_id, tag)
);

CREATE TABLE entry_links (
    from_entry_id INTEGER NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    to_entry_id INTEGER NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    relation TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (from_entry_id, to_entry_id, relation)
);

CREATE INDEX idx_entry_links_to ON entry_links(to_entry_id, relation);

CREATE TABLE entry_chunks (
    id INTEGER PRIMARY KEY,
    entry_id INTEGER NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    section TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    text TEXT NOT NULL,
    token_estimate INTEGER NOT NULL CHECK (token_estimate >= 0),
    content_hash TEXT NOT NULL,
    UNIQUE (entry_id, ordinal)
);

CREATE INDEX idx_entry_chunks_entry ON entry_chunks(entry_id, ordinal);

CREATE VIRTUAL TABLE entry_fts USING fts5(
    entry_id UNINDEXED,
    chunk_ordinal UNINDEXED,
    title,
    body,
    section,
    chunk_text,
    tokenize = 'unicode61'
);

CREATE TABLE sync_state (
    entry_id INTEGER PRIMARY KEY REFERENCES entries(id) ON DELETE CASCADE,
    source_path TEXT NOT NULL,
    sqlite_content_hash_at_last_sync TEXT NOT NULL,
    mirror_content_hash_at_last_sync TEXT NOT NULL,
    synced_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_sync_state_source_path ON sync_state(source_path);

CREATE TABLE agent_integrations (
    target TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    generated_version TEXT NOT NULL,
    installed_path TEXT,
    installed_hash TEXT,
    checked_at TEXT NOT NULL
);
"#;

const MIGRATION_2: &str = r#"
DROP TABLE entry_fts;

CREATE VIRTUAL TABLE entry_fts USING fts5(
    entry_id UNINDEXED,
    chunk_ordinal UNINDEXED,
    title,
    body,
    section,
    chunk_text,
    tokenize = 'unicode61'
);

INSERT INTO entry_fts(entry_id, chunk_ordinal, title, body, section, chunk_text)
SELECT chunks.entry_id, chunks.ordinal, entries.title, entries.body,
       chunks.section, chunks.text
FROM entry_chunks chunks
JOIN entries ON entries.id = chunks.entry_id;

INSERT INTO entry_fts(entry_id, chunk_ordinal, title, body, section, chunk_text)
SELECT entries.id, 0, entries.title, entries.body, 'Body', ''
FROM entries
WHERE NOT EXISTS (
    SELECT 1 FROM entry_chunks chunks WHERE chunks.entry_id = entries.id
);
"#;

pub fn initialize(path: &Path) -> Result<(), BelayError> {
    let mut connection =
        Connection::open(path).map_err(|source| BelayError::sqlite(path, source))?;
    configure(&connection, path)?;
    verify_fts5(&connection)?;
    migrate(&mut connection, path)
}

pub fn open(path: &Path) -> Result<Connection, BelayError> {
    let mut connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)
        .map_err(|source| BelayError::sqlite(path, source))?;
    configure(&connection, path)?;
    migrate(&mut connection, path)?;
    Ok(connection)
}

pub fn open_read_only(path: &Path) -> Result<Connection, BelayError> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|source| BelayError::sqlite(path, source))?;
    configure(&connection, path)?;
    let current = schema_version(&connection, path)?.unwrap_or(0);
    validate_supported_version(current)?;
    if current != LATEST_SCHEMA_VERSION {
        return Err(BelayError::Validation {
            message: format!(
                "database schema version {current} is not current; expected {LATEST_SCHEMA_VERSION}"
            ),
        });
    }
    Ok(connection)
}

pub fn current_schema_version(connection: &Connection, path: &Path) -> Result<i64, BelayError> {
    Ok(schema_version(connection, path)?.unwrap_or(0))
}

pub fn verify_schema_health(connection: &Connection, path: &Path) -> Result<(), BelayError> {
    let quick_check = connection
        .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
        .map_err(|source| BelayError::sqlite(path, source))?;
    if quick_check != "ok" {
        return Err(BelayError::Validation {
            message: format!("SQLite quick_check failed: {quick_check}"),
        });
    }

    let foreign_key_failure = connection
        .query_row(
            "SELECT 1 FROM pragma_foreign_key_check LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|source| BelayError::sqlite(path, source))?;
    if foreign_key_failure.is_some() {
        return Err(BelayError::Validation {
            message: "SQLite foreign_key_check found broken references".to_owned(),
        });
    }

    for (object_type, name) in [
        ("table", "entries"),
        ("table", "entry_tags"),
        ("table", "entry_links"),
        ("table", "entry_chunks"),
        ("table", "entry_fts"),
        ("table", "sync_state"),
        ("table", "agent_integrations"),
        ("index", "idx_entries_display_id"),
        ("index", "idx_entries_type_status"),
        ("index", "idx_entry_links_to"),
        ("index", "idx_entry_chunks_entry"),
        ("index", "idx_sync_state_source_path"),
    ] {
        let present = connection
            .query_row(
                "SELECT 1 FROM sqlite_schema WHERE type = ?1 AND name = ?2",
                params![object_type, name],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|source| BelayError::sqlite(path, source))?;
        if present.is_none() {
            return Err(BelayError::Validation {
                message: format!("SQLite schema object {object_type} {name} is missing"),
            });
        }
    }

    let fts_sql = connection
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE type = 'table' AND name = 'entry_fts'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map_err(|source| BelayError::sqlite(path, source))?;
    if !fts_sql.to_ascii_lowercase().contains("using fts5") {
        return Err(BelayError::Validation {
            message: "SQLite entry_fts is not an FTS5 virtual table".to_owned(),
        });
    }
    connection
        .query_row(
            "
            SELECT bm25(entry_fts)
            FROM entry_fts
            WHERE entry_fts MATCH 'belay_doctor_probe'
            LIMIT 1
            ",
            [],
            |row| row.get::<_, f64>(0),
        )
        .optional()
        .map_err(|source| BelayError::sqlite(path, source))?;
    Ok(())
}

fn configure(connection: &Connection, path: &Path) -> Result<(), BelayError> {
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|source| BelayError::sqlite(path, source))?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|source| BelayError::sqlite(path, source))
}

pub fn verify_fts5(connection: &Connection) -> Result<(), BelayError> {
    let result = (|| -> rusqlite::Result<Vec<(i64, f64)>> {
        connection.execute_batch(
            "
            CREATE VIRTUAL TABLE temp.belay_fts_capability USING fts5(content);
            INSERT INTO belay_fts_capability(content)
            VALUES ('sqlite sqlite trace storage'), ('sqlite unrelated content');
            ",
        )?;
        let mut statement = connection.prepare(
            "
            SELECT rowid, bm25(belay_fts_capability)
            FROM belay_fts_capability
            WHERE belay_fts_capability MATCH 'sqlite'
            ORDER BY bm25(belay_fts_capability)
            ",
        )?;
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()
    })();

    let _ = connection.execute_batch("DROP TABLE IF EXISTS temp.belay_fts_capability;");

    match result {
        Ok(results)
            if results.len() == 2
                && results.iter().all(|(_, score)| score.is_finite())
                && results[0].0 == 1
                && results[1].0 == 2
                && results[0].1 <= results[1].1 =>
        {
            Ok(())
        }
        Ok(results) => Err(BelayError::Capability {
            message: format!("bm25() returned invalid ranking results: {results:?}"),
        }),
        Err(source) => Err(BelayError::Capability {
            message: source.to_string(),
        }),
    }
}

pub fn migrate(connection: &mut Connection, path: &Path) -> Result<(), BelayError> {
    if let Some(current_version) = schema_version(connection, path)? {
        validate_supported_version(current_version)?;
        if current_version == LATEST_SCHEMA_VERSION {
            return Ok(());
        }
    }

    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|source| BelayError::sqlite(path, source))?;
    transaction
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );
            ",
        )
        .map_err(|source| BelayError::sqlite(path, source))?;

    let current_version: i64 = transaction
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|source| BelayError::sqlite(path, source))?;

    validate_supported_version(current_version)?;

    if current_version < 1 {
        transaction
            .execute_batch(MIGRATION_1)
            .map_err(|source| BelayError::sqlite(path, source))?;
        transaction
            .execute(
                "
                INSERT INTO schema_migrations(version, name, applied_at)
                VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                ",
                params![1, "initial v1 schema"],
            )
            .map_err(|source| BelayError::sqlite(path, source))?;
    }

    if current_version < 2 {
        transaction
            .execute_batch(MIGRATION_2)
            .map_err(|source| BelayError::sqlite(path, source))?;
        transaction
            .execute(
                "
                INSERT INTO schema_migrations(version, name, applied_at)
                VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                ",
                params![2, "add FTS chunk ordinals"],
            )
            .map_err(|source| BelayError::sqlite(path, source))?;
    }

    transaction
        .commit()
        .map_err(|source| BelayError::sqlite(path, source))
}

fn schema_version(connection: &Connection, path: &Path) -> Result<Option<i64>, BelayError> {
    let table_exists: bool = connection
        .query_row(
            "
            SELECT EXISTS(
                SELECT 1 FROM sqlite_schema
                WHERE type = 'table' AND name = 'schema_migrations'
            )
            ",
            [],
            |row| row.get(0),
        )
        .map_err(|source| BelayError::sqlite(path, source))?;
    if !table_exists {
        return Ok(None);
    }
    connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map(Some)
        .map_err(|source| BelayError::sqlite(path, source))
}

fn validate_supported_version(current_version: i64) -> Result<(), BelayError> {
    if current_version > LATEST_SCHEMA_VERSION {
        return Err(BelayError::Validation {
            message: format!(
                "database schema version {current_version} is newer than supported version {LATEST_SCHEMA_VERSION}"
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_sqlite_supports_fts5_and_bm25() {
        let connection = Connection::open_in_memory().expect("open in-memory SQLite");
        verify_fts5(&connection).expect("FTS5 and bm25 should be available");
    }

    #[test]
    fn migration_is_idempotent_and_uses_integer_entry_ids() {
        let mut connection = Connection::open_in_memory().expect("open in-memory SQLite");
        configure(&connection, Path::new(":memory:")).expect("configure SQLite");

        migrate(&mut connection, Path::new(":memory:")).expect("first migration");
        migrate(&mut connection, Path::new(":memory:")).expect("second migration");

        let migration_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("count migrations");
        assert_eq!(migration_count, 2);

        let entries_sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE type = 'table' AND name = 'entries'",
                [],
                |row| row.get(0),
            )
            .expect("read entries schema");
        assert!(entries_sql.contains("id INTEGER PRIMARY KEY"));
        assert!(!entries_sql.to_uppercase().contains("AUTOINCREMENT"));

        let unique_display_id_indexes: i64 = connection
            .query_row(
                "
                SELECT COUNT(*)
                FROM pragma_index_list('entries')
                WHERE name = 'idx_entries_display_id' AND \"unique\" = 1
                ",
                [],
                |row| row.get(0),
            )
            .expect("inspect display ID index");
        assert_eq!(unique_display_id_indexes, 1);

        let fts_sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE type = 'table' AND name = 'entry_fts'",
                [],
                |row| row.get(0),
            )
            .expect("read FTS schema");
        assert!(fts_sql.contains("chunk_ordinal UNINDEXED"));
    }

    #[test]
    fn migration_two_rebuilds_a_version_one_fts_index() {
        let mut connection = Connection::open_in_memory().expect("open in-memory SQLite");
        configure(&connection, Path::new(":memory:")).expect("configure SQLite");
        connection
            .execute_batch(
                "
                CREATE TABLE schema_migrations (
                    version INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    applied_at TEXT NOT NULL
                );
                ",
            )
            .expect("create migration table");
        connection
            .execute_batch(MIGRATION_1)
            .expect("create base schema");
        connection
            .execute_batch(
                "
                DROP TABLE entry_fts;
                CREATE VIRTUAL TABLE entry_fts USING fts5(
                    entry_id UNINDEXED,
                    title,
                    body,
                    section,
                    chunk_text,
                    tokenize = 'unicode61'
                );
                INSERT INTO schema_migrations(version, name, applied_at)
                VALUES (1, 'initial v1 schema', '2026-06-06T00:00:00Z');
                INSERT INTO entries(
                    display_id, type, title, status, created_at, updated_at,
                    revision, body, metadata_json, source_path, content_hash
                ) VALUES (
                    'DEC-20260606T000000-001-sqlite', 'decision', 'Use SQLite',
                    'proposed', '2026-06-06T00:00:00Z', '2026-06-06T00:00:00Z',
                    1, 'SQLite body', '{}', 'entries/decisions/example.md', 'hash'
                );
                INSERT INTO entry_chunks(
                    entry_id, section, ordinal, text, token_estimate, content_hash
                ) VALUES (1, 'Body', 0, 'SQLite body', 3, 'chunk-hash');
                INSERT INTO entry_fts(entry_id, title, body, section, chunk_text)
                VALUES (1, 'Use SQLite', 'SQLite body', 'Body', 'SQLite body');
                ",
            )
            .expect("create version one state");

        migrate(&mut connection, Path::new(":memory:")).expect("migrate to version two");

        let row: (i64, i64, String) = connection
            .query_row(
                "
                SELECT CAST(entry_id AS INTEGER), CAST(chunk_ordinal AS INTEGER), chunk_text
                FROM entry_fts
                WHERE entry_fts MATCH 'sqlite'
                ",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("query rebuilt FTS row");
        assert_eq!(row, (1, 0, "SQLite body".to_owned()));
    }
}
