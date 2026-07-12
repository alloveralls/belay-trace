use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use chrono::{DateTime, FixedOffset, NaiveDateTime, SecondsFormat};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};

use crate::error::BelayError;

pub const MARKDOWN_SCHEMA_VERSION: u32 = 1;
pub const MAX_SEQUENCE: u16 = 999;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    Goal,
    Plan,
    Decision,
    Work,
    Review,
    Note,
}

impl EntryType {
    pub const ALL: [Self; 6] = [
        Self::Goal,
        Self::Plan,
        Self::Decision,
        Self::Work,
        Self::Review,
        Self::Note,
    ];

    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Goal => "GOAL",
            Self::Plan => "PLN",
            Self::Decision => "DEC",
            Self::Work => "WRK",
            Self::Review => "REV",
            Self::Note => "NOTE",
        }
    }

    pub const fn directory(self) -> &'static str {
        match self {
            Self::Goal => "goals",
            Self::Plan => "plans",
            Self::Decision => "decisions",
            Self::Work => "work",
            Self::Review => "reviews",
            Self::Note => "notes",
        }
    }

    pub const fn default_status(self) -> EntryStatus {
        match self {
            Self::Goal => EntryStatus::Draft,
            Self::Plan => EntryStatus::Draft,
            Self::Decision => EntryStatus::Proposed,
            Self::Work => EntryStatus::InProgress,
            Self::Review => EntryStatus::Pending,
            Self::Note => EntryStatus::Active,
        }
    }

    pub const fn allows_status(self, status: EntryStatus) -> bool {
        match self {
            Self::Goal => matches!(
                status,
                EntryStatus::Draft
                    | EntryStatus::Active
                    | EntryStatus::Completed
                    | EntryStatus::Superseded
                    | EntryStatus::Abandoned
            ),
            Self::Plan => matches!(
                status,
                EntryStatus::Draft
                    | EntryStatus::Approved
                    | EntryStatus::Active
                    | EntryStatus::Completed
                    | EntryStatus::Superseded
                    | EntryStatus::Abandoned
            ),
            Self::Decision => matches!(
                status,
                EntryStatus::Proposed
                    | EntryStatus::Accepted
                    | EntryStatus::Rejected
                    | EntryStatus::Superseded
            ),
            Self::Work => matches!(
                status,
                EntryStatus::InProgress
                    | EntryStatus::Blocked
                    | EntryStatus::Completed
                    | EntryStatus::Abandoned
            ),
            Self::Review => matches!(status, EntryStatus::Pending | EntryStatus::Completed),
            Self::Note => matches!(status, EntryStatus::Active | EntryStatus::Archived),
        }
    }
}

impl fmt::Display for EntryType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Goal => "goal",
            Self::Plan => "plan",
            Self::Decision => "decision",
            Self::Work => "work",
            Self::Review => "review",
            Self::Note => "note",
        })
    }
}

impl FromStr for EntryType {
    type Err = BelayError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "goal" => Ok(Self::Goal),
            "plan" => Ok(Self::Plan),
            "decision" => Ok(Self::Decision),
            "work" => Ok(Self::Work),
            "review" => Ok(Self::Review),
            "note" => Ok(Self::Note),
            _ => Err(BelayError::Validation {
                message: format!(
                    "unsupported entry type {value:?}; expected goal, plan, decision, work, review, or note"
                ),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryStatus {
    Draft,
    Approved,
    Active,
    Completed,
    Superseded,
    Abandoned,
    Proposed,
    Accepted,
    Rejected,
    InProgress,
    Blocked,
    Pending,
    Archived,
}

impl fmt::Display for EntryStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Draft => "draft",
            Self::Approved => "approved",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Superseded => "superseded",
            Self::Abandoned => "abandoned",
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::InProgress => "in-progress",
            Self::Blocked => "blocked",
            Self::Pending => "pending",
            Self::Archived => "archived",
        })
    }
}

impl FromStr for EntryStatus {
    type Err = BelayError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "draft" => Ok(Self::Draft),
            "approved" => Ok(Self::Approved),
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            "superseded" => Ok(Self::Superseded),
            "abandoned" => Ok(Self::Abandoned),
            "proposed" => Ok(Self::Proposed),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            "in-progress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "pending" => Ok(Self::Pending),
            "archived" => Ok(Self::Archived),
            _ => validation(format!(
                "unsupported status {value:?}; use a status allowed by the entry type"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinkRelation {
    References,
    Implements,
    Reviews,
    Supersedes,
    FollowsUp,
    Fulfills,
    Supports,
    Verifies,
    Refutes,
}

impl fmt::Display for LinkRelation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::References => "references",
            Self::Implements => "implements",
            Self::Reviews => "reviews",
            Self::Supersedes => "supersedes",
            Self::FollowsUp => "follows-up",
            Self::Fulfills => "fulfills",
            Self::Supports => "supports",
            Self::Verifies => "verifies",
            Self::Refutes => "refutes",
        })
    }
}

impl FromStr for LinkRelation {
    type Err = BelayError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "references" => Ok(Self::References),
            "implements" => Ok(Self::Implements),
            "reviews" => Ok(Self::Reviews),
            "supersedes" => Ok(Self::Supersedes),
            "follows-up" => Ok(Self::FollowsUp),
            "fulfills" => Ok(Self::Fulfills),
            "supports" => Ok(Self::Supports),
            "verifies" => Ok(Self::Verifies),
            "refutes" => Ok(Self::Refutes),
            _ => validation(format!(
                "unsupported link relation {value:?}; expected references, implements, reviews, supersedes, follows-up, fulfills, supports, verifies, or refutes"
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetadataValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryLink {
    pub relation: LinkRelation,
    pub id: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, MetadataValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub display_id: String,
    pub entry_type: EntryType,
    pub title: String,
    pub status: EntryStatus,
    pub created_at: String,
    pub updated_at: String,
    pub revision: u32,
    pub tags: Vec<String>,
    pub links: Vec<EntryLink>,
    pub metadata: BTreeMap<String, MetadataValue>,
    pub body: String,
}

impl Entry {
    pub fn normalized(mut self) -> Result<Self, BelayError> {
        self.created_at = normalize_timestamp("created_at", &self.created_at)?;
        self.updated_at = normalize_timestamp("updated_at", &self.updated_at)?;
        self.body = normalize_newlines(&self.body)
            .trim_end_matches('\n')
            .to_owned();
        self.tags.sort();
        self.links.sort_by(|left, right| {
            (left.relation, left.id.as_str()).cmp(&(right.relation, right.id.as_str()))
        });
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), BelayError> {
        if self.title.trim().is_empty() {
            return validation("entry title must not be empty");
        }
        if self.title.contains('\0') || self.body.contains('\0') {
            return validation("entry title and body must not contain NUL characters");
        }
        if self.revision == 0 {
            return validation("entry revision must be at least 1");
        }
        if !self.entry_type.allows_status(self.status) {
            return validation(format!(
                "status {} is not valid for entry type {}",
                self.status, self.entry_type
            ));
        }

        let parts = parse_display_id(&self.display_id)?;
        if parts.entry_type != self.entry_type {
            return validation(format!(
                "display ID {} has type {}, but frontmatter type is {}",
                self.display_id, parts.entry_type, self.entry_type
            ));
        }

        let created = parse_timestamp("created_at", &self.created_at)?;
        let updated = parse_timestamp("updated_at", &self.updated_at)?;
        if updated < created {
            return validation("updated_at must not be earlier than created_at");
        }
        if parts.timestamp != created.format("%Y%m%dT%H%M%S").to_string() {
            return validation(format!(
                "display ID {} timestamp does not match created_at {}",
                self.display_id, self.created_at
            ));
        }

        let mut tags = BTreeSet::new();
        for tag in &self.tags {
            validate_name("tag", tag)?;
            if !tags.insert(tag) {
                return validation(format!("duplicate tag {tag:?}"));
            }
        }

        let mut links = BTreeSet::new();
        for link in &self.links {
            let reference = parse_entry_reference_id(&link.id)?;
            if reference.display_id == self.display_id {
                return validation("entry links must not target the same display ID");
            }
            if !links.insert((link.relation, link.id.as_str())) {
                return validation(format!("duplicate {} link to {}", link.relation, link.id));
            }
            validate_metadata(&link.metadata)?;
        }
        validate_metadata(&self.metadata)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayIdParts {
    pub entry_type: EntryType,
    pub timestamp: String,
    pub sequence: u16,
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryReferenceParts {
    pub display_id: String,
    pub fragment: Option<String>,
}

pub struct ImmediateTransaction<'connection> {
    transaction: Transaction<'connection>,
    database_path: PathBuf,
}

impl ImmediateTransaction<'_> {
    pub fn commit(self) -> Result<(), BelayError> {
        self.transaction
            .commit()
            .map_err(|source| BelayError::sqlite(self.database_path, source))
    }
}

impl<'connection> Deref for ImmediateTransaction<'connection> {
    type Target = Transaction<'connection>;

    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

pub fn begin_immediate<'connection>(
    connection: &'connection mut Connection,
    database_path: &Path,
) -> Result<ImmediateTransaction<'connection>, BelayError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|source| BelayError::sqlite(database_path, source))?;
    Ok(ImmediateTransaction {
        transaction,
        database_path: database_path.to_owned(),
    })
}

pub fn parse_display_id(display_id: &str) -> Result<DisplayIdParts, BelayError> {
    let mut parts = display_id.splitn(4, '-');
    let prefix = parts.next().unwrap_or_default();
    let timestamp = parts.next().unwrap_or_default();
    let sequence = parts.next().unwrap_or_default();
    let slug = parts.next().unwrap_or_default();

    let entry_type = match prefix {
        "GOAL" => EntryType::Goal,
        "PLN" => EntryType::Plan,
        "DEC" => EntryType::Decision,
        "WRK" => EntryType::Work,
        "REV" => EntryType::Review,
        "NOTE" => EntryType::Note,
        _ => return invalid_display_id(display_id),
    };
    NaiveDateTime::parse_from_str(timestamp, "%Y%m%dT%H%M%S")
        .map_err(|_| invalid_display_id_error(display_id))?;
    let sequence_value = sequence
        .parse::<u16>()
        .ok()
        .filter(|value| (1..=MAX_SEQUENCE).contains(value))
        .ok_or_else(|| invalid_display_id_error(display_id))?;
    if sequence.len() != 3 || sequence != format!("{sequence_value:03}") || !valid_slug(slug) {
        return invalid_display_id(display_id);
    }

    Ok(DisplayIdParts {
        entry_type,
        timestamp: timestamp.to_owned(),
        sequence: sequence_value,
        slug: slug.to_owned(),
    })
}

pub fn parse_entry_reference_id(value: &str) -> Result<EntryReferenceParts, BelayError> {
    let (display_id, fragment) = match value.split_once('#') {
        Some((display_id, fragment)) => {
            if fragment.is_empty()
                || !fragment
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
            {
                return Err(BelayError::Validation {
                    message: format!(
                        "invalid entry reference {value:?}; fragment must be ASCII letters, digits, or hyphen"
                    ),
                });
            }
            (display_id, Some(fragment.to_owned()))
        }
        None => (value, None),
    };
    parse_display_id(display_id)?;
    Ok(EntryReferenceParts {
        display_id: display_id.to_owned(),
        fragment,
    })
}

fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && slug
            .chars()
            .all(|character| character.is_alphanumeric() || character == '-')
}

pub fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut pending_separator = false;

    for character in title.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if pending_separator && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(character);
            pending_separator = false;
        } else {
            pending_separator = true;
        }
        if slug.chars().count() >= 48 {
            break;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "entry".to_owned()
    } else {
        slug
    }
}

pub fn allocate_display_id(
    transaction: &ImmediateTransaction<'_>,
    entries_root: &Path,
    entry_type: EntryType,
    created_at: &DateTime<FixedOffset>,
    title: &str,
) -> Result<String, BelayError> {
    if title.trim().is_empty() {
        return validation("entry title must not be empty before display-ID allocation");
    }

    let timestamp = created_at.format("%Y%m%dT%H%M%S").to_string();
    let slug = slugify(title);
    let mirror_names = managed_mirror_names(entries_root)?;

    for sequence in 1..=MAX_SEQUENCE {
        let sequence_prefix = format!("{}-{timestamp}-{sequence:03}-", entry_type.prefix());
        let normalized_prefix = sequence_prefix.to_ascii_lowercase();
        let database_collision = transaction
            .query_row(
                "SELECT display_id FROM entries WHERE display_id GLOB ?1 LIMIT 1",
                params![format!("{sequence_prefix}*")],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|source| BelayError::sqlite(&transaction.database_path, source))?
            .is_some();
        let mirror_collision = mirror_names.iter().any(|name| {
            name.to_str()
                .map(str::to_ascii_lowercase)
                .is_some_and(|name| name.starts_with(&normalized_prefix) && name.ends_with(".md"))
        });
        if database_collision || mirror_collision {
            continue;
        }
        return Ok(format!("{sequence_prefix}{slug}"));
    }

    validation(format!(
        "more than {MAX_SEQUENCE} {} entries already exist for {timestamp}; retry in a later second",
        entry_type
    ))
}

fn managed_mirror_names(entries_root: &Path) -> Result<Vec<std::ffi::OsString>, BelayError> {
    let mut names = Vec::new();
    for entry_type in EntryType::ALL {
        let directory = entries_root.join(entry_type.directory());
        for entry in fs::read_dir(&directory)
            .map_err(|source| BelayError::io("read managed entry directory", &directory, source))?
        {
            names.push(
                entry
                    .map_err(|source| {
                        BelayError::io("read managed entry directory", &directory, source)
                    })?
                    .file_name(),
            );
        }
    }
    Ok(names)
}

pub(crate) fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn normalize_timestamp(field: &str, value: &str) -> Result<String, BelayError> {
    Ok(parse_timestamp(field, value)?.to_rfc3339_opts(SecondsFormat::Secs, false))
}

fn parse_timestamp(field: &str, value: &str) -> Result<DateTime<FixedOffset>, BelayError> {
    DateTime::parse_from_rfc3339(value).map_err(|source| BelayError::Validation {
        message: format!("{field} must be an RFC 3339 timestamp: {source}"),
    })
}

fn validate_name(kind: &str, value: &str) -> Result<(), BelayError> {
    if value.is_empty() || value.trim() != value || value.contains('\0') {
        return validation(format!(
            "{kind} names must be non-empty, trimmed, and contain no NUL characters"
        ));
    }
    Ok(())
}

fn validate_metadata(metadata: &BTreeMap<String, MetadataValue>) -> Result<(), BelayError> {
    for (key, value) in metadata {
        validate_name("metadata key", key)?;
        if matches!(value, MetadataValue::Float(number) if !number.is_finite()) {
            return validation(format!("metadata value for {key:?} must be finite"));
        }
    }
    Ok(())
}

fn invalid_display_id(display_id: &str) -> Result<DisplayIdParts, BelayError> {
    Err(invalid_display_id_error(display_id))
}

fn invalid_display_id_error(display_id: &str) -> BelayError {
    BelayError::Validation {
        message: format!(
            "invalid display ID {display_id:?}; expected TYPE-YYYYMMDDTHHmmss-NNN-slug"
        ),
    }
}

fn validation<T>(message: impl Into<String>) -> Result<T, BelayError> {
    Err(BelayError::Validation {
        message: message.into(),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};
    use std::thread;

    use chrono::DateTime;
    use rusqlite::Connection;
    use tempfile::tempdir;

    use crate::database;

    use super::*;

    fn create_entry_directories(entries_root: &Path) {
        for entry_type in EntryType::ALL {
            fs::create_dir_all(entries_root.join(entry_type.directory()))
                .expect("create mirror directory");
        }
    }

    #[test]
    fn type_specific_statuses_are_validated() {
        assert_eq!(EntryType::Decision.default_status(), EntryStatus::Proposed);
        assert!(EntryType::Decision.allows_status(EntryStatus::Accepted));
        assert!(!EntryType::Decision.allows_status(EntryStatus::InProgress));
    }

    #[test]
    fn slug_generation_is_stable_and_has_a_fallback() {
        assert_eq!(
            slugify("Use SQLite: Runtime Store"),
            "use-sqlite-runtime-store"
        );
        assert_eq!(slugify("日本語の計画"), "日本語の計画");
        assert_eq!(slugify("---"), "entry");
    }

    #[test]
    fn display_id_parser_rejects_noncanonical_sequences() {
        let valid =
            parse_display_id("DEC-20260606T120000-001-use-sqlite").expect("valid display ID");
        assert_eq!(valid.entry_type, EntryType::Decision);
        assert_eq!(valid.sequence, 1);

        for invalid in [
            "DEC-20260606T120000-1-use-sqlite",
            "DEC-20260606T120000-000-use-sqlite",
            "DEC-20260606T120000-001--use-sqlite",
            "DEC-20260606T120000-001-use/sqlite",
        ] {
            assert!(parse_display_id(invalid).is_err(), "{invalid} must fail");
        }
    }

    #[test]
    fn allocator_skips_database_and_mirror_collisions() {
        let temporary = tempdir().expect("create temp directory");
        let database_path = temporary.path().join("belay.sqlite");
        let entries_root = temporary.path().join("entries");
        create_entry_directories(&entries_root);
        database::initialize(&database_path).expect("initialize database");
        fs::write(
            entries_root.join("plans/DEC-20260606T120000-002-existing-file.MD"),
            "collision",
        )
        .expect("write collision");

        let mut connection = database::open(&database_path).expect("open database");
        connection
            .execute(
                "
                INSERT INTO entries(
                    display_id, type, title, status, created_at, updated_at, revision
                ) VALUES (?1, 'decision', 'Existing', 'proposed', ?2, ?2, 1)
                ",
                params![
                    "DEC-20260606T120000-001-existing-row",
                    "2026-06-06T12:00:00+09:00"
                ],
            )
            .expect("insert collision");
        let transaction =
            begin_immediate(&mut connection, &database_path).expect("begin immediate transaction");
        let timestamp =
            DateTime::parse_from_rfc3339("2026-06-06T12:00:00+09:00").expect("parse timestamp");
        let display_id = allocate_display_id(
            &transaction,
            &entries_root,
            EntryType::Decision,
            &timestamp,
            "New decision",
        )
        .expect("allocate ID");
        assert_eq!(display_id, "DEC-20260606T120000-003-new-decision");
    }

    #[test]
    fn concurrent_allocators_reserve_distinct_sequences() {
        let temporary = tempdir().expect("create temp directory");
        let database_path = temporary.path().join("belay.sqlite");
        let entries_root = temporary.path().join("entries");
        create_entry_directories(&entries_root);
        database::initialize(&database_path).expect("initialize database");

        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();
        for title in ["First", "Second"] {
            let barrier = Arc::clone(&barrier);
            let database_path = database_path.clone();
            let entries_root = entries_root.clone();
            handles.push(thread::spawn(move || {
                let mut connection = database::open(&database_path).expect("open database");
                barrier.wait();
                let transaction = begin_immediate(&mut connection, &database_path)
                    .expect("begin immediate transaction");
                let timestamp = DateTime::parse_from_rfc3339("2026-06-06T12:00:00+09:00")
                    .expect("parse timestamp");
                let display_id = allocate_display_id(
                    &transaction,
                    &entries_root,
                    EntryType::Decision,
                    &timestamp,
                    title,
                )
                .expect("allocate display ID");
                transaction
                    .execute(
                        "
                        INSERT INTO entries(
                            display_id, type, title, status, created_at, updated_at, revision
                        ) VALUES (?1, 'decision', ?2, 'proposed', ?3, ?3, 1)
                        ",
                        params![display_id, title, "2026-06-06T12:00:00+09:00"],
                    )
                    .expect("reserve display ID");
                transaction.commit().expect("commit reservation");
                display_id
            }));
        }
        barrier.wait();

        let mut ids = handles
            .into_iter()
            .map(|handle| handle.join().expect("join allocator"))
            .collect::<Vec<_>>();
        ids.sort();
        assert!(ids[0].contains("-001-"));
        assert!(ids[1].contains("-002-"));

        let connection = Connection::open(database_path).expect("open database");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
            .expect("count entries");
        assert_eq!(count, 2);
    }
}
