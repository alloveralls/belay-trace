#[cfg(test)]
use std::cell::Cell;
use std::collections::BTreeMap;
#[cfg(unix)]
use std::ffi::OsString;
use std::fs;
#[cfg(not(unix))]
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, FixedOffset, Local, SecondsFormat};
use rusqlite::{Connection, OptionalExtension, Row, params};
#[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
use rustix::fs::RenameFlags;
#[cfg(unix)]
use rustix::fs::{AtFlags, Mode, OFlags};
#[cfg(unix)]
use std::os::fd::OwnedFd;

use crate::database;
use crate::entry::{
    Entry, EntryLink, EntryStatus, EntryType, LinkRelation, MetadataValue, allocate_display_id,
    begin_immediate, parse_display_id,
};
use crate::error::BelayError;
use crate::markdown::{self, EntryChunk};
use crate::repository::Repository;

#[derive(Debug)]
pub struct ShownEntry {
    pub entry: Entry,
    pub source_path: String,
    pub inbound_links: Vec<EntryLink>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationOutcome {
    Changed,
    Unchanged,
}

#[cfg(test)]
thread_local! {
    static FAIL_DIRECTORY_SYNC: Cell<bool> = const { Cell::new(false) };
}

pub fn create(
    repository: &Repository,
    entry_type: EntryType,
    title: String,
    body: String,
) -> Result<Entry, BelayError> {
    if title.trim().is_empty() {
        return validation("entry title must not be empty");
    }
    if title.contains('\0') || body.contains('\0') {
        return validation("entry title and body must not contain NUL characters");
    }

    let now = now();
    let timestamp = format_timestamp(&now);
    let database_path = repository.database_path();
    let mut connection = database::open(&database_path)?;
    let transaction = begin_immediate(&mut connection, &database_path)?;
    let display_id = allocate_display_id(
        &transaction,
        &repository.entries_path(),
        entry_type,
        &now,
        &title,
    )?;
    let entry = Entry {
        display_id,
        entry_type,
        title,
        status: entry_type.default_status(),
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
        revision: 1,
        tags: Vec::new(),
        links: Vec::new(),
        metadata: BTreeMap::new(),
        body,
    }
    .normalized()?;
    let relative_path = mirror_relative_path(repository, &entry);
    let source_path = path_to_storage_string(&relative_path)?;
    let destination = repository.belay_dir.join(&relative_path);
    let rendered = markdown::render(&entry)?;
    let hash = markdown::content_hash(&entry)?;
    let chunks = markdown::generate_chunks(&entry.body);

    transaction
        .execute(
            "
            INSERT INTO entries(
                display_id, type, title, status, created_at, updated_at, revision,
                body, metadata_json, source_path, content_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ",
            params![
                entry.display_id,
                entry.entry_type.to_string(),
                entry.title,
                entry.status.to_string(),
                entry.created_at,
                entry.updated_at,
                entry.revision,
                entry.body,
                serialize_metadata(&entry.metadata)?,
                source_path,
                hash,
            ],
        )
        .map_err(|source| BelayError::sqlite(&database_path, source))?;
    let internal_id = transaction.last_insert_rowid();
    replace_tags(&transaction, &database_path, internal_id, &entry.tags)?;
    replace_chunks_and_fts(&transaction, &database_path, internal_id, &entry, &chunks)?;
    upsert_sync_state(
        &transaction,
        &database_path,
        internal_id,
        &source_path,
        &hash,
        &timestamp,
    )?;
    write_new_file(repository, &destination, rendered.as_bytes())?;
    transaction.commit()?;
    Ok(entry)
}

pub fn show(repository: &Repository, display_id: &str) -> Result<ShownEntry, BelayError> {
    parse_display_id(display_id)?;
    let database_path = repository.database_path();
    let connection = database::open(&database_path)?;
    load_shown_entry(&connection, &database_path, display_id)
}

pub fn link(
    repository: &Repository,
    from: &str,
    to: &str,
    relation: LinkRelation,
) -> Result<MutationOutcome, BelayError> {
    parse_display_id(from)?;
    parse_display_id(to)?;
    if from == to {
        return validation("entry links must not target the same display ID");
    }

    let database_path = repository.database_path();
    let mut connection = database::open(&database_path)?;
    let transaction = begin_immediate(&mut connection, &database_path)?;
    let from_id = resolve_internal_id(&transaction, &database_path, from)?;
    let to_id = resolve_internal_id(&transaction, &database_path, to)?;
    let expected_mirror_hash =
        ensure_no_mirror_drift(repository, &transaction, &database_path, from_id)?;
    let inserted = transaction
        .execute(
            "
            INSERT OR IGNORE INTO entry_links(
                from_entry_id, to_entry_id, relation, metadata_json
            ) VALUES (?1, ?2, ?3, '{}')
            ",
            params![from_id, to_id, relation.to_string()],
        )
        .map_err(|source| BelayError::sqlite(&database_path, source))?;
    if inserted == 0 {
        return Ok(MutationOutcome::Unchanged);
    }

    update_mutated_entry(
        repository,
        &transaction,
        &database_path,
        from_id,
        None,
        &expected_mirror_hash,
    )?;
    transaction.commit()?;
    Ok(MutationOutcome::Changed)
}

pub fn set_status(
    repository: &Repository,
    display_id: &str,
    status: EntryStatus,
) -> Result<MutationOutcome, BelayError> {
    parse_display_id(display_id)?;
    let database_path = repository.database_path();
    let mut connection = database::open(&database_path)?;
    let transaction = begin_immediate(&mut connection, &database_path)?;
    let internal_id = resolve_internal_id(&transaction, &database_path, display_id)?;
    let entry_type = transaction
        .query_row(
            "SELECT type FROM entries WHERE id = ?1",
            [internal_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|source| BelayError::sqlite(&database_path, source))?
        .parse::<EntryType>()?;
    if !entry_type.allows_status(status) {
        return validation(format!(
            "status {status} is not valid for entry type {entry_type}"
        ));
    }
    let current = transaction
        .query_row(
            "SELECT status FROM entries WHERE id = ?1",
            [internal_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|source| BelayError::sqlite(&database_path, source))?
        .parse::<EntryStatus>()?;
    let expected_mirror_hash =
        ensure_no_mirror_drift(repository, &transaction, &database_path, internal_id)?;
    if current == status {
        return Ok(MutationOutcome::Unchanged);
    }

    transaction
        .execute(
            "UPDATE entries SET status = ?1 WHERE id = ?2",
            params![status.to_string(), internal_id],
        )
        .map_err(|source| BelayError::sqlite(&database_path, source))?;
    update_mutated_entry(
        repository,
        &transaction,
        &database_path,
        internal_id,
        Some(status),
        &expected_mirror_hash,
    )?;
    transaction.commit()?;
    Ok(MutationOutcome::Changed)
}

fn update_mutated_entry(
    repository: &Repository,
    connection: &Connection,
    database_path: &Path,
    internal_id: i64,
    status: Option<EntryStatus>,
    expected_mirror_hash: &str,
) -> Result<(), BelayError> {
    let mut entry = load_entry(connection, database_path, internal_id)?;
    if let Some(status) = status {
        entry.status = status;
    }
    entry.revision = entry
        .revision
        .checked_add(1)
        .ok_or_else(|| BelayError::Validation {
            message: format!("entry {} revision overflowed", entry.display_id),
        })?;
    entry.updated_at = mutation_timestamp(&entry.created_at)?;
    entry = entry.normalized()?;
    let hash = markdown::content_hash(&entry)?;
    let rendered = markdown::render(&entry)?;
    let source_path: String = connection
        .query_row(
            "SELECT source_path FROM entries WHERE id = ?1",
            [internal_id],
            |row| row.get(0),
        )
        .map_err(|source| BelayError::sqlite(database_path, source))?;
    let destination = managed_source_path(repository, &source_path)?;

    connection
        .execute(
            "
            UPDATE entries
            SET status = ?1, updated_at = ?2, revision = ?3, content_hash = ?4
            WHERE id = ?5
            ",
            params![
                entry.status.to_string(),
                entry.updated_at,
                entry.revision,
                hash,
                internal_id
            ],
        )
        .map_err(|source| BelayError::sqlite(database_path, source))?;
    upsert_sync_state(
        connection,
        database_path,
        internal_id,
        &source_path,
        &hash,
        &entry.updated_at,
    )?;
    replace_file(
        repository,
        &destination,
        rendered.as_bytes(),
        expected_mirror_hash,
    )?;
    Ok(())
}

fn ensure_no_mirror_drift(
    repository: &Repository,
    connection: &Connection,
    database_path: &Path,
    internal_id: i64,
) -> Result<String, BelayError> {
    let (display_id, source_path, sqlite_hash, baseline_sqlite_hash, baseline_mirror_hash) =
        connection
            .query_row(
                "
                SELECT entry.display_id, entry.source_path, entry.content_hash,
                       state.sqlite_content_hash_at_last_sync,
                       state.mirror_content_hash_at_last_sync
                FROM entries entry
                JOIN sync_state state ON state.entry_id = entry.id
                WHERE entry.id = ?1
                ",
                [internal_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .map_err(|source| BelayError::sqlite(database_path, source))?;
    if sqlite_hash != baseline_sqlite_hash {
        return Err(BelayError::Conflict {
            message: format!(
                "entry {display_id} has unsynchronized SQLite changes; run `belay sync` before mutating it"
            ),
        });
    }

    let mirror_path = managed_source_path(repository, &source_path)?;
    let contents = read_managed_file(repository, &mirror_path)?;
    let mirror = markdown::parse(&contents).map_err(|error| BelayError::Conflict {
        message: format!(
            "entry {display_id} has invalid or unsynchronized Markdown ({error}); run `belay sync` before mutating it"
        ),
    })?;
    if mirror.display_id != display_id {
        return Err(BelayError::Conflict {
            message: format!(
                "entry {display_id} mirror contains display ID {}; run `belay sync` before mutating it",
                mirror.display_id
            ),
        });
    }
    let mirror_hash = markdown::content_hash(&mirror)?;
    if mirror_hash != baseline_mirror_hash {
        return Err(BelayError::Conflict {
            message: format!(
                "entry {display_id} has unsynchronized Markdown changes; run `belay sync` before mutating it"
            ),
        });
    }
    Ok(baseline_mirror_hash)
}

fn load_shown_entry(
    connection: &Connection,
    database_path: &Path,
    display_id: &str,
) -> Result<ShownEntry, BelayError> {
    let internal_id = resolve_internal_id(connection, database_path, display_id)?;
    let entry = load_entry(connection, database_path, internal_id)?;
    let source_path = connection
        .query_row(
            "SELECT source_path FROM entries WHERE id = ?1",
            [internal_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|source| BelayError::sqlite(database_path, source))?;
    let inbound_links = load_links(connection, database_path, internal_id, false)?;
    Ok(ShownEntry {
        entry,
        source_path,
        inbound_links,
    })
}

pub(crate) fn load_entry(
    connection: &Connection,
    database_path: &Path,
    internal_id: i64,
) -> Result<Entry, BelayError> {
    let mut entry = connection
        .query_row(
            "
            SELECT display_id, type, title, status, created_at, updated_at,
                   revision, body, metadata_json
            FROM entries
            WHERE id = ?1
            ",
            [internal_id],
            entry_from_row,
        )
        .map_err(|source| BelayError::sqlite(database_path, source))?;
    entry.tags = load_tags(connection, database_path, internal_id)?;
    entry.links = load_links(connection, database_path, internal_id, true)?;
    entry.normalized()
}

fn entry_from_row(row: &Row<'_>) -> rusqlite::Result<Entry> {
    let entry_type = row.get::<_, String>(1)?;
    let status = row.get::<_, String>(3)?;
    let metadata = row.get::<_, String>(8)?;
    Ok(Entry {
        display_id: row.get(0)?,
        entry_type: entry_type.parse().map_err(conversion_error)?,
        title: row.get(2)?,
        status: status.parse().map_err(conversion_error)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        revision: row.get(6)?,
        tags: Vec::new(),
        links: Vec::new(),
        metadata: serde_json::from_str(&metadata).map_err(conversion_error)?,
        body: row.get(7)?,
    })
}

fn load_tags(
    connection: &Connection,
    database_path: &Path,
    internal_id: i64,
) -> Result<Vec<String>, BelayError> {
    let mut statement = connection
        .prepare("SELECT tag FROM entry_tags WHERE entry_id = ?1 ORDER BY tag")
        .map_err(|source| BelayError::sqlite(database_path, source))?;
    statement
        .query_map([internal_id], |row| row.get(0))
        .map_err(|source| BelayError::sqlite(database_path, source))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|source| BelayError::sqlite(database_path, source))
}

fn load_links(
    connection: &Connection,
    database_path: &Path,
    internal_id: i64,
    outbound: bool,
) -> Result<Vec<EntryLink>, BelayError> {
    let sql = if outbound {
        "
        SELECT links.relation, target.display_id, links.metadata_json
        FROM entry_links links
        JOIN entries target ON target.id = links.to_entry_id
        WHERE links.from_entry_id = ?1
        ORDER BY links.relation, target.display_id
        "
    } else {
        "
        SELECT links.relation, source.display_id, links.metadata_json
        FROM entry_links links
        JOIN entries source ON source.id = links.from_entry_id
        WHERE links.to_entry_id = ?1
        ORDER BY links.relation, source.display_id
        "
    };
    let mut statement = connection
        .prepare(sql)
        .map_err(|source| BelayError::sqlite(database_path, source))?;
    statement
        .query_map([internal_id], |row| {
            let relation = row.get::<_, String>(0)?;
            let metadata = row.get::<_, String>(2)?;
            Ok(EntryLink {
                relation: relation.parse().map_err(conversion_error)?,
                id: row.get(1)?,
                metadata: serde_json::from_str(&metadata).map_err(conversion_error)?,
            })
        })
        .map_err(|source| BelayError::sqlite(database_path, source))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|source| BelayError::sqlite(database_path, source))
}

pub(crate) fn resolve_internal_id(
    connection: &Connection,
    database_path: &Path,
    display_id: &str,
) -> Result<i64, BelayError> {
    connection
        .query_row(
            "SELECT id FROM entries WHERE display_id = ?1",
            [display_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|source| BelayError::sqlite(database_path, source))?
        .ok_or_else(|| BelayError::Validation {
            message: format!("entry {display_id} was not found"),
        })
}

pub(crate) fn replace_tags(
    connection: &Connection,
    database_path: &Path,
    internal_id: i64,
    tags: &[String],
) -> Result<(), BelayError> {
    for tag in tags {
        connection
            .execute(
                "INSERT INTO entry_tags(entry_id, tag) VALUES (?1, ?2)",
                params![internal_id, tag],
            )
            .map_err(|source| BelayError::sqlite(database_path, source))?;
    }
    Ok(())
}

pub(crate) fn replace_chunks_and_fts(
    connection: &Connection,
    database_path: &Path,
    internal_id: i64,
    entry: &Entry,
    chunks: &[EntryChunk],
) -> Result<(), BelayError> {
    connection
        .execute(
            "DELETE FROM entry_chunks WHERE entry_id = ?1",
            [internal_id],
        )
        .map_err(|source| BelayError::sqlite(database_path, source))?;
    connection
        .execute("DELETE FROM entry_fts WHERE entry_id = ?1", [internal_id])
        .map_err(|source| BelayError::sqlite(database_path, source))?;

    for chunk in chunks {
        let token_estimate =
            i64::try_from(chunk.token_estimate).map_err(|_| BelayError::Validation {
                message: format!(
                    "chunk {} token estimate exceeds SQLite integer range",
                    chunk.ordinal
                ),
            })?;
        connection
            .execute(
                "
                INSERT INTO entry_chunks(
                    entry_id, section, ordinal, text, token_estimate, content_hash
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
                params![
                    internal_id,
                    chunk.section,
                    chunk.ordinal,
                    chunk.text,
                    token_estimate,
                    chunk.content_hash,
                ],
            )
            .map_err(|source| BelayError::sqlite(database_path, source))?;
        connection
            .execute(
                "
                INSERT INTO entry_fts(
                    entry_id, chunk_ordinal, title, body, section, chunk_text
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
                params![
                    internal_id,
                    chunk.ordinal,
                    entry.title,
                    entry.body,
                    chunk.section,
                    chunk.text
                ],
            )
            .map_err(|source| BelayError::sqlite(database_path, source))?;
    }
    if chunks.is_empty() {
        connection
            .execute(
                "
                INSERT INTO entry_fts(
                    entry_id, chunk_ordinal, title, body, section, chunk_text
                )
                VALUES (?1, 0, ?2, ?3, 'Body', '')
                ",
                params![internal_id, entry.title, entry.body],
            )
            .map_err(|source| BelayError::sqlite(database_path, source))?;
    }
    Ok(())
}

pub(crate) fn upsert_sync_state(
    connection: &Connection,
    database_path: &Path,
    internal_id: i64,
    source_path: &str,
    hash: &str,
    synced_at: &str,
) -> Result<(), BelayError> {
    connection
        .execute(
            "
            INSERT INTO sync_state(
                entry_id, source_path, sqlite_content_hash_at_last_sync,
                mirror_content_hash_at_last_sync, synced_at
            ) VALUES (?1, ?2, ?3, ?3, ?4)
            ON CONFLICT(entry_id) DO UPDATE SET
                source_path = excluded.source_path,
                sqlite_content_hash_at_last_sync =
                    excluded.sqlite_content_hash_at_last_sync,
                mirror_content_hash_at_last_sync =
                    excluded.mirror_content_hash_at_last_sync,
                synced_at = excluded.synced_at
            ",
            params![internal_id, source_path, hash, synced_at],
        )
        .map_err(|source| BelayError::sqlite(database_path, source))?;
    Ok(())
}

pub(crate) fn insert_entry(
    connection: &Connection,
    database_path: &Path,
    entry: &Entry,
    source_path: &str,
) -> Result<i64, BelayError> {
    let hash = markdown::content_hash(entry)?;
    connection
        .execute(
            "
            INSERT INTO entries(
                display_id, type, title, status, created_at, updated_at, revision,
                body, metadata_json, source_path, content_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ",
            params![
                entry.display_id,
                entry.entry_type.to_string(),
                entry.title,
                entry.status.to_string(),
                entry.created_at,
                entry.updated_at,
                entry.revision,
                entry.body,
                serialize_metadata(&entry.metadata)?,
                source_path,
                hash,
            ],
        )
        .map_err(|source| BelayError::sqlite(database_path, source))?;
    let internal_id = connection.last_insert_rowid();
    replace_tags(connection, database_path, internal_id, &entry.tags)?;
    let chunks = markdown::generate_chunks(&entry.body);
    replace_chunks_and_fts(connection, database_path, internal_id, entry, &chunks)?;
    Ok(internal_id)
}

pub(crate) fn replace_entry(
    connection: &Connection,
    database_path: &Path,
    internal_id: i64,
    entry: &Entry,
    source_path: &str,
) -> Result<(), BelayError> {
    let hash = markdown::content_hash(entry)?;
    connection
        .execute(
            "
            UPDATE entries
            SET type = ?1, title = ?2, status = ?3, created_at = ?4,
                updated_at = ?5, revision = ?6, body = ?7, metadata_json = ?8,
                source_path = ?9, content_hash = ?10
            WHERE id = ?11
            ",
            params![
                entry.entry_type.to_string(),
                entry.title,
                entry.status.to_string(),
                entry.created_at,
                entry.updated_at,
                entry.revision,
                entry.body,
                serialize_metadata(&entry.metadata)?,
                source_path,
                hash,
                internal_id,
            ],
        )
        .map_err(|source| BelayError::sqlite(database_path, source))?;
    connection
        .execute("DELETE FROM entry_tags WHERE entry_id = ?1", [internal_id])
        .map_err(|source| BelayError::sqlite(database_path, source))?;
    replace_tags(connection, database_path, internal_id, &entry.tags)?;
    let chunks = markdown::generate_chunks(&entry.body);
    replace_chunks_and_fts(connection, database_path, internal_id, entry, &chunks)
}

pub(crate) fn replace_links(
    connection: &Connection,
    database_path: &Path,
    internal_id: i64,
    links: &[EntryLink],
) -> Result<(), BelayError> {
    connection
        .execute(
            "DELETE FROM entry_links WHERE from_entry_id = ?1",
            [internal_id],
        )
        .map_err(|source| BelayError::sqlite(database_path, source))?;
    for link in links {
        let target_id = resolve_internal_id(connection, database_path, &link.id)?;
        let metadata =
            serde_json::to_string(&link.metadata).map_err(|source| BelayError::Validation {
                message: format!("could not serialize link metadata: {source}"),
            })?;
        connection
            .execute(
                "
                INSERT INTO entry_links(
                    from_entry_id, to_entry_id, relation, metadata_json
                ) VALUES (?1, ?2, ?3, ?4)
                ",
                params![internal_id, target_id, link.relation.to_string(), metadata],
            )
            .map_err(|source| BelayError::sqlite(database_path, source))?;
    }
    Ok(())
}

pub(crate) fn serialize_metadata(
    metadata: &BTreeMap<String, MetadataValue>,
) -> Result<String, BelayError> {
    serde_json::to_string(metadata).map_err(|source| BelayError::Validation {
        message: format!("could not serialize entry metadata: {source}"),
    })
}

pub(crate) fn mirror_relative_path(repository: &Repository, entry: &Entry) -> PathBuf {
    repository
        .config
        .storage
        .entries
        .join(entry.entry_type.directory())
        .join(format!("{}.md", entry.display_id))
}

pub(crate) fn managed_source_path(
    repository: &Repository,
    source_path: &str,
) -> Result<PathBuf, BelayError> {
    let relative = Path::new(source_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || !relative.starts_with(&repository.config.storage.entries)
    {
        return validation(format!(
            "entry source path {source_path:?} is outside the managed mirror"
        ));
    }
    Ok(repository.belay_dir.join(relative))
}

pub(crate) fn path_to_storage_string(path: &Path) -> Result<String, BelayError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| BelayError::Validation {
            message: format!("managed source path {} is not valid UTF-8", path.display()),
        })
}

pub(crate) fn write_new_file(
    repository: &Repository,
    path: &Path,
    contents: &[u8],
) -> Result<(), BelayError> {
    #[cfg(unix)]
    {
        let parent = open_managed_parent(repository, path)?;
        let temporary = write_temporary_at(&parent, path, contents)?;
        match rustix::fs::linkat(
            &parent.fd,
            &temporary,
            &parent.fd,
            &parent.file_name,
            AtFlags::empty(),
        ) {
            Ok(()) => {
                rustix::fs::unlinkat(&parent.fd, &temporary, AtFlags::empty())
                    .map_err(|source| unix_io("remove temporary file", path, source))?;
                sync_managed_directory(&parent)?;
                Ok(())
            }
            Err(source) => {
                let _ = rustix::fs::unlinkat(&parent.fd, &temporary, AtFlags::empty());
                Err(unix_io("create managed Markdown", path, source))
            }
        }
    }

    #[cfg(not(unix))]
    {
        write_new_file_portable(repository, path, contents)
    }
}

#[cfg(not(unix))]
fn write_new_file_portable(
    repository: &Repository,
    path: &Path,
    contents: &[u8],
) -> Result<(), BelayError> {
    validate_managed_parent(repository, path)?;
    if fs::symlink_metadata(path).is_ok() {
        return validation(format!(
            "managed Markdown path {} already exists; no file was overwritten",
            path.display()
        ));
    }
    let temporary = write_temporary(repository, path, contents)?;
    match fs::hard_link(&temporary, path) {
        Ok(()) => {
            fs::remove_file(&temporary)
                .map_err(|source| BelayError::io("remove temporary file", &temporary, source))?;
            sync_parent_directory(repository, path)?;
            Ok(())
        }
        Err(source) => {
            let _ = fs::remove_file(&temporary);
            Err(BelayError::io("create managed Markdown", path, source))
        }
    }
}

pub(crate) fn replace_file(
    repository: &Repository,
    path: &Path,
    contents: &[u8],
    expected_hash: &str,
) -> Result<(), BelayError> {
    #[cfg(unix)]
    {
        let parent = open_managed_parent(repository, path)?;
        validate_regular_file_at(&parent, path)?;
        let temporary = write_temporary_at(&parent, path, contents)?;
        publish_replacement(&parent, path, &temporary, expected_hash)
    }

    #[cfg(not(unix))]
    {
        replace_file_portable(repository, path, contents, expected_hash)
    }
}

#[cfg(not(unix))]
fn replace_file_portable(
    repository: &Repository,
    path: &Path,
    contents: &[u8],
    expected_hash: &str,
) -> Result<(), BelayError> {
    validate_managed_file(repository, path)?;
    let current = fs::read_to_string(path)
        .map_err(|source| BelayError::io("read managed Markdown", path, source))?;
    ensure_expected_mirror(path, &current, expected_hash)?;
    let temporary = write_temporary(repository, path, contents)?;
    if let Err(source) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(BelayError::io("replace managed Markdown", path, source));
    }
    sync_parent_directory(repository, path)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_temporary(
    repository: &Repository,
    destination: &Path,
    contents: &[u8],
) -> Result<PathBuf, BelayError> {
    validate_managed_parent(repository, destination)?;
    let parent = destination.parent().ok_or_else(|| BelayError::Validation {
        message: format!(
            "managed Markdown path {} has no parent",
            destination.display()
        ),
    })?;
    let file_name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| BelayError::Validation {
            message: format!(
                "managed Markdown path {} has an invalid filename",
                destination.display()
            ),
        })?;
    for attempt in 0..100_u32 {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary = parent.join(format!(
            ".{file_name}.tmp-{}-{nonce}-{attempt}",
            std::process::id()
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(mut file) => {
                file.write_all(contents)
                    .map_err(|source| BelayError::io("write temporary file", &temporary, source))?;
                file.sync_all()
                    .map_err(|source| BelayError::io("flush temporary file", &temporary, source))?;
                return Ok(temporary);
            }
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => {
                return Err(BelayError::io("create temporary file", &temporary, source));
            }
        }
    }
    validation(format!(
        "could not allocate a temporary file beside {}",
        destination.display()
    ))
}

#[cfg(not(unix))]
fn validate_managed_file(repository: &Repository, path: &Path) -> Result<(), BelayError> {
    validate_managed_parent(repository, path)?;
    let metadata =
        fs::symlink_metadata(path).map_err(|source| BelayError::io("inspect", path, source))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return validation(format!(
            "managed Markdown path {} must be a regular file",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_managed_parent(repository: &Repository, path: &Path) -> Result<(), BelayError> {
    let relative =
        path.strip_prefix(&repository.belay_dir)
            .map_err(|_| BelayError::Validation {
                message: format!(
                    "managed path {} must stay within {}",
                    path.display(),
                    repository.belay_dir.display()
                ),
            })?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return validation(format!(
            "managed path {} must use normal relative components",
            path.display()
        ));
    }

    require_real_directory(&repository.belay_dir)?;
    let mut current = repository.belay_dir.clone();
    let parent = relative.parent().ok_or_else(|| BelayError::Validation {
        message: format!("managed path {} has no parent", path.display()),
    })?;
    for component in parent.components() {
        let Component::Normal(component) = component else {
            return validation(format!(
                "managed path {} must use normal relative components",
                path.display()
            ));
        };
        current.push(component);
        require_real_directory(&current)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn require_real_directory(path: &Path) -> Result<(), BelayError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|source| BelayError::io("inspect", path, source))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return validation(format!(
            "managed path {} must be a real directory, not a symbolic link",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn sync_parent_directory(repository: &Repository, path: &Path) -> Result<(), BelayError> {
    validate_managed_parent(repository, path)?;
    Ok(())
}

#[cfg(unix)]
struct ManagedParent {
    fd: OwnedFd,
    file_name: OsString,
    path: PathBuf,
}

#[cfg(unix)]
fn open_managed_parent(repository: &Repository, path: &Path) -> Result<ManagedParent, BelayError> {
    let relative =
        path.strip_prefix(&repository.belay_dir)
            .map_err(|_| BelayError::Validation {
                message: format!(
                    "managed path {} must stay within {}",
                    path.display(),
                    repository.belay_dir.display()
                ),
            })?;
    let file_name = relative
        .file_name()
        .ok_or_else(|| BelayError::Validation {
            message: format!("managed path {} has no filename", path.display()),
        })?
        .to_os_string();
    let parent = relative.parent().ok_or_else(|| BelayError::Validation {
        message: format!("managed path {} has no parent", path.display()),
    })?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return validation(format!(
            "managed path {} must use normal relative components",
            path.display()
        ));
    }

    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut fd = rustix::fs::open(&repository.belay_dir, directory_flags, Mode::empty())
        .map_err(|source| managed_directory_error(&repository.belay_dir, source))?;
    let mut parent_path = repository.belay_dir.clone();
    for component in parent.components() {
        let Component::Normal(component) = component else {
            return validation(format!(
                "managed path {} must use normal relative components",
                path.display()
            ));
        };
        parent_path.push(component);
        fd = rustix::fs::openat(&fd, component, directory_flags, Mode::empty())
            .map_err(|source| managed_directory_error(&parent_path, source))?;
    }
    Ok(ManagedParent {
        fd,
        file_name,
        path: parent_path,
    })
}

#[cfg(unix)]
fn validate_regular_file_at(parent: &ManagedParent, path: &Path) -> Result<(), BelayError> {
    let fd = rustix::fs::openat(
        &parent.fd,
        &parent.file_name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|source| unix_io("open managed Markdown", path, source))?;
    let file = fs::File::from(fd);
    let metadata = file
        .metadata()
        .map_err(|source| BelayError::io("inspect", path, source))?;
    if !metadata.is_file() {
        return validation(format!(
            "managed Markdown path {} must be a regular file",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn read_managed_file(
    repository: &Repository,
    path: &Path,
) -> Result<String, BelayError> {
    let parent = open_managed_parent(repository, path)?;
    let fd = rustix::fs::openat(
        &parent.fd,
        &parent.file_name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|source| unix_io("open managed Markdown", path, source))?;
    let mut file = fs::File::from(fd);
    let metadata = file
        .metadata()
        .map_err(|source| BelayError::io("inspect", path, source))?;
    if !metadata.is_file() {
        return validation(format!(
            "managed Markdown path {} must be a regular file",
            path.display()
        ));
    }
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|source| BelayError::io("read managed Markdown", path, source))?;
    Ok(contents)
}

#[cfg(not(unix))]
pub(crate) fn read_managed_file(
    repository: &Repository,
    path: &Path,
) -> Result<String, BelayError> {
    validate_managed_file(repository, path)?;
    fs::read_to_string(path).map_err(|source| BelayError::io("read managed Markdown", path, source))
}

#[cfg(unix)]
fn write_temporary_at(
    parent: &ManagedParent,
    destination: &Path,
    contents: &[u8],
) -> Result<OsString, BelayError> {
    let file_name = parent
        .file_name
        .to_str()
        .ok_or_else(|| BelayError::Validation {
            message: format!(
                "managed Markdown path {} has an invalid filename",
                destination.display()
            ),
        })?;
    for attempt in 0..100_u32 {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary = OsString::from(format!(
            ".{file_name}.tmp-{}-{nonce}-{attempt}",
            std::process::id()
        ));
        match rustix::fs::openat(
            &parent.fd,
            &temporary,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR | Mode::RGRP | Mode::ROTH,
        ) {
            Ok(fd) => {
                let mut file = fs::File::from(fd);
                file.write_all(contents).map_err(|source| {
                    BelayError::io("write temporary file", destination, source)
                })?;
                file.sync_all().map_err(|source| {
                    BelayError::io("flush temporary file", destination, source)
                })?;
                return Ok(temporary);
            }
            Err(rustix::io::Errno::EXIST) => continue,
            Err(source) => {
                return Err(unix_io("create temporary file", destination, source));
            }
        }
    }
    validation(format!(
        "could not allocate a temporary file beside {}",
        destination.display()
    ))
}

#[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
fn publish_replacement(
    parent: &ManagedParent,
    path: &Path,
    temporary: &OsString,
    expected_hash: &str,
) -> Result<(), BelayError> {
    if let Err(source) = rustix::fs::renameat_with(
        &parent.fd,
        temporary,
        &parent.fd,
        &parent.file_name,
        RenameFlags::EXCHANGE,
    ) {
        let _ = rustix::fs::unlinkat(&parent.fd, temporary, AtFlags::empty());
        return Err(unix_io("exchange managed Markdown", path, source));
    }

    let previous = read_file_at(parent, temporary, path);
    let expected = match previous {
        Ok(contents) => ensure_expected_mirror(path, &contents, expected_hash),
        Err(error) => Err(error),
    };
    if let Err(error) = expected {
        let rollback = rustix::fs::renameat_with(
            &parent.fd,
            temporary,
            &parent.fd,
            &parent.file_name,
            RenameFlags::EXCHANGE,
        );
        if rollback.is_ok() {
            let _ = rustix::fs::unlinkat(&parent.fd, temporary, AtFlags::empty());
            let _ = sync_managed_directory(parent);
            return Err(error);
        }
        return Err(BelayError::Conflict {
            message: format!(
                "managed Markdown {} changed concurrently and rollback failed; preserve both files and run `belay sync`",
                path.display()
            ),
        });
    }

    rustix::fs::unlinkat(&parent.fd, temporary, AtFlags::empty())
        .map_err(|source| unix_io("remove previous managed Markdown", path, source))?;
    sync_managed_directory(parent)
}

#[cfg(all(
    unix,
    not(any(target_vendor = "apple", target_os = "linux", target_os = "android"))
))]
fn publish_replacement(
    parent: &ManagedParent,
    path: &Path,
    temporary: &OsString,
    expected_hash: &str,
) -> Result<(), BelayError> {
    let current = read_file_at(parent, &parent.file_name, path)?;
    ensure_expected_mirror(path, &current, expected_hash)?;
    if let Err(source) = rustix::fs::renameat(&parent.fd, temporary, &parent.fd, &parent.file_name)
    {
        let _ = rustix::fs::unlinkat(&parent.fd, temporary, AtFlags::empty());
        return Err(unix_io("replace managed Markdown", path, source));
    }
    sync_managed_directory(parent)
}

#[cfg(unix)]
fn read_file_at(
    parent: &ManagedParent,
    file_name: &std::ffi::OsStr,
    path: &Path,
) -> Result<String, BelayError> {
    let fd = rustix::fs::openat(
        &parent.fd,
        file_name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|source| unix_io("open managed Markdown", path, source))?;
    let mut file = fs::File::from(fd);
    let metadata = file
        .metadata()
        .map_err(|source| BelayError::io("inspect", path, source))?;
    if !metadata.is_file() {
        return validation(format!(
            "managed Markdown path {} must be a regular file",
            path.display()
        ));
    }
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|source| BelayError::io("read managed Markdown", path, source))?;
    Ok(contents)
}

fn ensure_expected_mirror(
    path: &Path,
    contents: &str,
    expected_hash: &str,
) -> Result<(), BelayError> {
    let entry = markdown::parse(contents).map_err(|error| BelayError::Conflict {
        message: format!(
            "managed Markdown {} changed concurrently ({error}); run `belay sync`",
            path.display()
        ),
    })?;
    let actual_hash = markdown::content_hash(&entry)?;
    if actual_hash != expected_hash {
        return Err(BelayError::Conflict {
            message: format!(
                "managed Markdown {} changed concurrently; run `belay sync`",
                path.display()
            ),
        });
    }
    Ok(())
}

#[cfg(unix)]
fn unix_io(action: &'static str, path: &Path, source: rustix::io::Errno) -> BelayError {
    BelayError::io(
        action,
        path,
        std::io::Error::from_raw_os_error(source.raw_os_error()),
    )
}

#[cfg(unix)]
fn sync_managed_directory(parent: &ManagedParent) -> Result<(), BelayError> {
    #[cfg(test)]
    if FAIL_DIRECTORY_SYNC.replace(false) {
        return Err(BelayError::io(
            "flush directory",
            &parent.path,
            std::io::Error::other("injected directory sync failure"),
        ));
    }
    rustix::fs::fsync(&parent.fd).map_err(|source| unix_io("flush directory", &parent.path, source))
}

#[cfg(unix)]
fn managed_directory_error(path: &Path, source: rustix::io::Errno) -> BelayError {
    if matches!(source, rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR) {
        BelayError::Validation {
            message: format!(
                "managed path {} must be a real directory, not a symbolic link",
                path.display()
            ),
        }
    } else {
        unix_io("open managed directory", path, source)
    }
}

fn now() -> DateTime<FixedOffset> {
    Local::now().fixed_offset()
}

fn mutation_timestamp(created_at: &str) -> Result<String, BelayError> {
    let created =
        DateTime::parse_from_rfc3339(created_at).map_err(|source| BelayError::Validation {
            message: format!("created_at must be an RFC 3339 timestamp: {source}"),
        })?;
    let current = now();
    Ok(format_timestamp(if current < created {
        &created
    } else {
        &current
    }))
}

fn format_timestamp(timestamp: &DateTime<FixedOffset>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::Secs, false)
}

fn conversion_error(error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn validation<T>(message: impl Into<String>) -> Result<T, BelayError> {
    Err(BelayError::Validation {
        message: message.into(),
    })
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;

    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::*;
    use crate::repository;

    #[test]
    fn directory_sync_failure_leaves_markdown_for_recovery_and_rolls_back_sqlite() {
        let temporary = tempdir().expect("create temp directory");
        fs::create_dir(temporary.path().join(".git")).expect("create repository marker");
        let repository = repository::initialize(temporary.path())
            .expect("initialize repository")
            .repository;

        FAIL_DIRECTORY_SYNC.set(true);
        let error = create(
            &repository,
            EntryType::Decision,
            "Recoverable decision".to_owned(),
            "Body".to_owned(),
        )
        .expect_err("injected directory sync must fail");
        assert_eq!(error.exit_code(), 6);

        let connection = Connection::open(repository.database_path()).expect("open database");
        let entry_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
            .expect("count entries");
        assert_eq!(entry_count, 0);

        let mirrors = fs::read_dir(repository.entries_path().join("decisions"))
            .expect("read mirrors")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect mirrors");
        assert_eq!(mirrors.len(), 1);
        let rendered = fs::read_to_string(mirrors[0].path()).expect("read recoverable mirror");
        let entry = markdown::parse(&rendered).expect("parse recoverable mirror");
        assert_eq!(entry.title, "Recoverable decision");

        let replacement_temporary = tempdir().expect("create replacement temp directory");
        fs::create_dir(replacement_temporary.path().join(".git"))
            .expect("create replacement repository marker");
        let replacement_repository = repository::initialize(replacement_temporary.path())
            .expect("initialize replacement repository")
            .repository;
        let created = create(
            &replacement_repository,
            EntryType::Decision,
            "Replacement decision".to_owned(),
            "Body".to_owned(),
        )
        .expect("create replacement entry");

        FAIL_DIRECTORY_SYNC.set(true);
        let error = set_status(
            &replacement_repository,
            &created.display_id,
            EntryStatus::Accepted,
        )
        .expect_err("injected replacement directory sync must fail");
        assert_eq!(error.exit_code(), 6);

        let connection =
            Connection::open(replacement_repository.database_path()).expect("open replacement DB");
        let database_state: (String, i64) = connection
            .query_row(
                "SELECT status, revision FROM entries WHERE display_id = ?1",
                [&created.display_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read rolled-back database state");
        assert_eq!(database_state, ("proposed".to_owned(), 1));

        let mirror_path = replacement_repository
            .entries_path()
            .join("decisions")
            .join(format!("{}.md", created.display_id));
        let mirror = markdown::parse(
            &fs::read_to_string(mirror_path).expect("read replacement recovery mirror"),
        )
        .expect("parse replacement recovery mirror");
        assert_eq!(mirror.status, EntryStatus::Accepted);
        assert_eq!(mirror.revision, 2);
    }

    #[test]
    fn replacement_restores_the_original_when_expected_mirror_hash_does_not_match() {
        let temporary = tempdir().expect("create temp directory");
        fs::create_dir(temporary.path().join(".git")).expect("create repository marker");
        let repository = repository::initialize(temporary.path())
            .expect("initialize repository")
            .repository;
        let created = create(
            &repository,
            EntryType::Decision,
            "Concurrent decision".to_owned(),
            "Original body".to_owned(),
        )
        .expect("create entry");
        let mirror_path = repository
            .entries_path()
            .join("decisions")
            .join(format!("{}.md", created.display_id));
        let original = fs::read_to_string(&mirror_path).expect("read original mirror");
        let mut replacement = created;
        replacement.status = EntryStatus::Accepted;
        replacement.revision = 2;
        let rendered = markdown::render(&replacement).expect("render replacement");

        let error = replace_file(
            &repository,
            &mirror_path,
            rendered.as_bytes(),
            "unexpected-hash",
        )
        .expect_err("mismatched expected hash must fail");
        assert_eq!(error.exit_code(), 5);
        assert_eq!(
            fs::read_to_string(&mirror_path).expect("read restored mirror"),
            original
        );
    }
}
