use std::collections::BTreeMap;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use sha2::{Digest, Sha256};

use crate::entry::{
    Entry, EntryLink, EntryStatus, EntryType, MARKDOWN_SCHEMA_VERSION, MetadataValue,
    normalize_newlines,
};
use crate::error::BelayError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryChunk {
    pub section: String,
    pub ordinal: u32,
    pub text: String,
    pub token_estimate: usize,
    pub content_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Frontmatter {
    schema_version: u32,
    id: String,
    #[serde(rename = "type")]
    entry_type: EntryType,
    title: String,
    status: EntryStatus,
    created_at: String,
    updated_at: String,
    revision: u32,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    links: Vec<EntryLink>,
    #[serde(default)]
    metadata: BTreeMap<String, MetadataValue>,
}

#[derive(Serialize)]
struct HashableEntry<'a> {
    schema_version: u32,
    id: &'a str,
    #[serde(rename = "type")]
    entry_type: EntryType,
    title: &'a str,
    status: EntryStatus,
    created_at: &'a str,
    tags: &'a [String],
    links: &'a [EntryLink],
    metadata: &'a BTreeMap<String, MetadataValue>,
    body: &'a str,
}

pub fn parse(input: &str) -> Result<Entry, BelayError> {
    let normalized = normalize_newlines(input);
    let remainder = normalized
        .strip_prefix("---\n")
        .ok_or_else(|| validation_error("managed Markdown must start with `---` frontmatter"))?;
    let closing = remainder
        .find("\n---\n")
        .ok_or_else(|| validation_error("managed Markdown frontmatter must end with `---`"))?;
    let yaml = &remainder[..closing];
    let mut body = &remainder[closing + "\n---\n".len()..];
    if let Some(without_separator) = body.strip_prefix('\n') {
        body = without_separator;
    }
    body = body.strip_suffix('\n').unwrap_or(body);

    let value: Value = serde_yaml::from_str(yaml).map_err(yaml_error)?;
    validate_schema_version(&value)?;
    let frontmatter: Frontmatter = serde_yaml::from_value(value).map_err(yaml_error)?;

    Entry {
        display_id: frontmatter.id,
        entry_type: frontmatter.entry_type,
        title: frontmatter.title,
        status: frontmatter.status,
        created_at: frontmatter.created_at,
        updated_at: frontmatter.updated_at,
        revision: frontmatter.revision,
        tags: frontmatter.tags,
        links: frontmatter.links,
        metadata: frontmatter.metadata,
        body: body.to_owned(),
    }
    .normalized()
}

pub fn render(entry: &Entry) -> Result<String, BelayError> {
    let entry = entry.clone().normalized()?;
    let frontmatter = Frontmatter {
        schema_version: MARKDOWN_SCHEMA_VERSION,
        id: entry.display_id,
        entry_type: entry.entry_type,
        title: entry.title,
        status: entry.status,
        created_at: entry.created_at,
        updated_at: entry.updated_at,
        revision: entry.revision,
        tags: entry.tags,
        links: entry.links,
        metadata: entry.metadata,
    };
    let yaml = serde_yaml::to_string(&frontmatter).map_err(yaml_error)?;
    let yaml = yaml.strip_prefix("---\n").unwrap_or(&yaml);
    let body = entry.body;
    if body.is_empty() {
        Ok(format!("---\n{yaml}---\n\n"))
    } else {
        Ok(format!("---\n{yaml}---\n\n{body}\n"))
    }
}

pub fn content_hash(entry: &Entry) -> Result<String, BelayError> {
    let entry = entry.clone().normalized()?;
    let hashable = HashableEntry {
        schema_version: MARKDOWN_SCHEMA_VERSION,
        id: &entry.display_id,
        entry_type: entry.entry_type,
        title: &entry.title,
        status: entry.status,
        created_at: &entry.created_at,
        tags: &entry.tags,
        links: &entry.links,
        metadata: &entry.metadata,
        body: &entry.body,
    };
    let canonical = serde_yaml::to_string(&hashable).map_err(yaml_error)?;
    Ok(sha256_hex(canonical.as_bytes()))
}

pub fn generate_chunks(body: &str) -> Vec<EntryChunk> {
    let normalized = normalize_newlines(body);
    let parser = Parser::new_ext(&normalized, Options::all()).into_offset_iter();
    let mut chunks = Vec::new();
    let mut section = "Body".to_owned();
    let mut section_start = 0;
    let mut heading = String::new();
    let mut in_heading = false;
    let mut heading_end = None;

    for (event, range) in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                push_chunk(
                    &mut chunks,
                    &section,
                    &normalized[section_start..range.start],
                );
                heading.clear();
                in_heading = true;
                heading_end = Some(range.end);
            }
            Event::End(TagEnd::Heading(_)) => {
                section = if heading.trim().is_empty() {
                    "Untitled".to_owned()
                } else {
                    heading.trim().to_owned()
                };
                in_heading = false;
                section_start = heading_end.take().unwrap_or(range.end);
            }
            Event::Text(value)
            | Event::Code(value)
            | Event::InlineHtml(value)
            | Event::FootnoteReference(value) => {
                if in_heading {
                    heading.push_str(&value);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_heading {
                    heading.push(' ');
                }
            }
            Event::Html(_)
            | Event::Rule
            | Event::TaskListMarker(_)
            | Event::Start(_)
            | Event::End(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_) => {}
        }
    }
    push_chunk(&mut chunks, &section, &normalized[section_start..]);
    chunks
}

pub fn estimate_tokens(text: &str) -> usize {
    let ascii_bytes = text.bytes().filter(u8::is_ascii).count();
    let non_ascii_scalars = text
        .chars()
        .filter(|character| !character.is_ascii())
        .count();
    ascii_bytes.div_ceil(4) + non_ascii_scalars
}

fn validate_schema_version(value: &Value) -> Result<(), BelayError> {
    let mapping = value
        .as_mapping()
        .ok_or_else(|| validation_error("frontmatter must be a YAML mapping"))?;
    let version = mapping_value(mapping, "schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| validation_error("frontmatter schema_version must be an integer"))?;
    match version {
        value if value == u64::from(MARKDOWN_SCHEMA_VERSION) => Ok(()),
        value if value > u64::from(MARKDOWN_SCHEMA_VERSION) => Err(validation_error(format!(
            "Markdown schema version {value} is newer than supported version {MARKDOWN_SCHEMA_VERSION}"
        ))),
        value => Err(validation_error(format!(
            "Markdown schema version {value} is unsupported; expected {MARKDOWN_SCHEMA_VERSION}"
        ))),
    }
}

fn mapping_value<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_owned()))
}

fn push_chunk(chunks: &mut Vec<EntryChunk>, section: &str, text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    let ordinal = chunks.len() as u32;
    let hash_input = format!("{section}\0{text}");
    chunks.push(EntryChunk {
        section: section.to_owned(),
        ordinal,
        text: text.to_owned(),
        token_estimate: estimate_tokens(text),
        content_hash: sha256_hex(hash_input.as_bytes()),
    });
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

fn yaml_error(source: serde_yaml::Error) -> BelayError {
    BelayError::Validation {
        message: format!("invalid managed Markdown frontmatter: {source}"),
    }
}

fn validation_error(message: impl Into<String>) -> BelayError {
    BelayError::Validation {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::entry::{EntryLink, LinkRelation};

    fn sample_entry() -> Entry {
        Entry {
            display_id: "DEC-20260606T120000-001-use-sqlite".to_owned(),
            entry_type: EntryType::Decision,
            title: "Use SQLite as operational store".to_owned(),
            status: EntryStatus::Proposed,
            created_at: "2026-06-06T12:00:00+09:00".to_owned(),
            updated_at: "2026-06-06T12:00:00+09:00".to_owned(),
            revision: 1,
            tags: vec!["storage".to_owned(), "architecture".to_owned()],
            links: vec![EntryLink {
                relation: LinkRelation::References,
                id: "PLN-20260606T115959-001-v1-plan".to_owned(),
                metadata: BTreeMap::new(),
            }],
            metadata: BTreeMap::from([
                ("approved".to_owned(), MetadataValue::Boolean(false)),
                ("priority".to_owned(), MetadataValue::Integer(1)),
                ("reviewer".to_owned(), MetadataValue::Null),
                (
                    "scope".to_owned(),
                    MetadataValue::String("local".to_owned()),
                ),
            ]),
            body: "Intro\r\n\r\n## Rationale\r\n\r\nFast **local** retrieval.\r\n".to_owned(),
        }
    }

    #[test]
    fn render_is_deterministic_and_round_trips_without_internal_ids() {
        let rendered = render(&sample_entry()).expect("render entry");
        let reparsed = parse(&rendered).expect("parse rendered entry");
        let rendered_again = render(&reparsed).expect("render reparsed entry");

        assert_eq!(rendered, rendered_again);
        assert_eq!(
            reparsed,
            sample_entry().normalized().expect("normalize sample")
        );
        assert!(!rendered.contains("entry_id"));
        assert!(!rendered.contains("content_hash"));

        let expected_order = [
            "schema_version:",
            "id:",
            "type:",
            "title:",
            "status:",
            "created_at:",
            "updated_at:",
            "revision:",
            "tags:",
            "links:",
            "metadata:",
        ];
        let positions = expected_order.map(|key| rendered.find(key).expect("frontmatter key"));
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn hash_uses_normalized_content_and_ignores_managed_revision_fields() {
        let original = sample_entry();
        let mut equivalent = sample_entry();
        equivalent.body = equivalent.body.replace("\r\n", "\n");
        equivalent.revision = 12;
        equivalent.updated_at = "2026-06-06T13:00:00+09:00".to_owned();
        equivalent.tags.reverse();

        assert_eq!(
            content_hash(&original).expect("hash original"),
            content_hash(&equivalent).expect("hash equivalent")
        );

        equivalent.title.push_str(" v2");
        assert_ne!(
            content_hash(&original).expect("hash original"),
            content_hash(&equivalent).expect("hash changed")
        );
    }

    #[test]
    fn parser_rejects_nested_metadata_and_future_schema() {
        let rendered = render(&sample_entry()).expect("render entry");
        let nested = rendered.replace("metadata:\n", "metadata:\n  nested:\n    child: value\n");
        assert!(parse(&nested).is_err());

        let future = rendered.replacen("schema_version: 1", "schema_version: 2", 1);
        let error = parse(&future).expect_err("future schema must fail");
        assert!(error.to_string().contains("newer than supported"));
    }

    #[test]
    fn parser_rejects_metadata_arrays_and_unknown_fields() {
        let rendered = render(&sample_entry()).expect("render entry");
        let array = rendered.replace("metadata:\n", "metadata:\n  nested: [one, two]\n");
        assert!(parse(&array).is_err());

        let unknown = rendered.replace("revision: 1\n", "revision: 1\ninternal_id: 42\n");
        assert!(parse(&unknown).is_err());
    }

    #[test]
    fn parser_preserves_display_identity_for_rebuilds() {
        let rendered = render(&sample_entry()).expect("render entry");
        let parsed = parse(&rendered).expect("parse entry");
        assert_eq!(parsed.display_id, "DEC-20260606T120000-001-use-sqlite");
    }

    #[test]
    fn chunks_follow_markdown_sections_and_use_documented_estimator() {
        let chunks = generate_chunks(
            "Preamble text.\n\n## First section\n\nASCII text.\n\n### 日本語\n\n日本語 text.",
        );
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].section, "Body");
        assert_eq!(chunks[1].section, "First section");
        assert_eq!(chunks[2].section, "日本語");
        assert_eq!(chunks[0].ordinal, 0);
        assert_eq!(chunks[2].ordinal, 2);
        assert_eq!(estimate_tokens("abcd日本"), 3);
        assert_eq!(chunks[2].token_estimate, estimate_tokens(&chunks[2].text));
        assert!(chunks.iter().all(|chunk| chunk.content_hash.len() == 64));
    }

    #[test]
    fn chunks_preserve_paragraph_and_list_boundaries() {
        let chunks = generate_chunks("## Details\n\nalpha\n\nbeta\n\n- first item\n- second item");
        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0].text,
            "alpha\n\nbeta\n\n- first item\n- second item"
        );

        let flattened = generate_chunks("## Details\n\nalphabeta\n\n- first item- second item");
        assert_ne!(chunks[0].content_hash, flattened[0].content_hash);
    }
}
