use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use crate::entry::EntryType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceSpan {
    pub start: usize,
    pub end: usize,
    pub value: String,
    pub evidence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalIdFinding {
    pub field: &'static str,
    pub message: String,
    pub line: usize,
}

pub fn fragment_exists(entry_type: EntryType, body: &str, fragment: &str) -> bool {
    fragment_match_count(entry_type, body, fragment) == 1
}

pub fn local_fragments(entry_type: EntryType, body: &str) -> BTreeSet<String> {
    match entry_type {
        EntryType::Goal => goal_fragments(body),
        EntryType::Plan => plan_fragments(body),
        _ => BTreeSet::new(),
    }
}

pub fn fragment_match_count(entry_type: EntryType, body: &str, fragment: &str) -> usize {
    match entry_type {
        EntryType::Goal => goal_items(body)
            .into_iter()
            .enumerate()
            .filter(|(ordinal, (_, item))| {
                format!("sc-{}", ordinal + 1).eq_ignore_ascii_case(fragment)
                    || goal_item_fragments(item)
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(fragment))
            })
            .count(),
        EntryType::Plan => delivery_map_ids(body)
            .into_iter()
            .filter(|id| {
                let normalized = id.to_ascii_lowercase();
                normalized.eq_ignore_ascii_case(fragment)
                    || format!("task-{normalized}").eq_ignore_ascii_case(fragment)
            })
            .count(),
        _ => 0,
    }
}

pub fn goal_id_findings(body: &str) -> Vec<LocalIdFinding> {
    let mut findings = Vec::new();
    let mut seen = BTreeSet::new();
    for (line, item) in goal_items(body) {
        let Some(id) = explicit_goal_id(item) else {
            findings.push(LocalIdFinding {
                field: "Success Criteria",
                message: "criterion must start with a stable ID such as `[SC-001]`".to_owned(),
                line,
            });
            continue;
        };
        let normalized = id.to_ascii_lowercase();
        if !seen.insert(normalized) {
            findings.push(LocalIdFinding {
                field: "Success Criteria",
                message: format!("duplicate criterion ID {id}"),
                line,
            });
        }
    }
    findings
}

pub fn plan_id_findings(body: &str) -> Vec<LocalIdFinding> {
    let mut findings = Vec::new();
    let mut seen = BTreeSet::new();
    for (line, id) in delivery_map_id_rows(body) {
        if !valid_task_id(&id) {
            findings.push(LocalIdFinding {
                field: "Delivery Map",
                message: format!("task ID {id:?} must use fixed-width form such as `T-001`"),
                line,
            });
            continue;
        }
        if !seen.insert(id.to_ascii_lowercase()) {
            findings.push(LocalIdFinding {
                field: "Delivery Map",
                message: format!("duplicate task ID {id}"),
                line,
            });
        }
    }
    findings
}

pub fn valid_goal_id(value: &str) -> bool {
    valid_numbered_id(value, "SC-")
}

pub fn valid_task_id(value: &str) -> bool {
    valid_numbered_id(value, "T-")
}

pub fn reference_spans(text: &str) -> Vec<ReferenceSpan> {
    let mut spans = Vec::new();
    let mut cursor = 0;
    while cursor < text.len() {
        let Some((start, _)) = text[cursor..]
            .char_indices()
            .find(|(_, character)| character.is_ascii_uppercase())
        else {
            break;
        };
        let start = cursor + start;
        let previous_is_id = text[..start].chars().next_back().is_some_and(id_character);
        if previous_is_id {
            cursor = next_character(text, start);
            continue;
        }
        let end = text[start..]
            .char_indices()
            .find_map(|(offset, character)| {
                (!id_character(character) && character != '#').then_some(start + offset)
            })
            .unwrap_or(text.len());
        let candidate = &text[start..end];
        let entry = crate::entry::parse_entry_reference_id(candidate)
            .ok()
            .filter(|reference| {
                reference.fragment.as_deref().is_none_or(|fragment| {
                    let upper = fragment.to_ascii_uppercase();
                    valid_goal_id(&upper) || valid_task_id(&upper)
                })
            });
        let evidence = valid_evidence_id(candidate);
        if entry.is_some() || evidence {
            spans.push(ReferenceSpan {
                start,
                end,
                value: candidate.to_owned(),
                evidence,
            });
            cursor = end;
        } else {
            cursor = next_character(text, start);
        }
    }
    spans
}

fn valid_numbered_id(value: &str, prefix: &str) -> bool {
    let Some(number) = value.strip_prefix(prefix) else {
        return false;
    };
    number.len() == 3 && number.bytes().all(|byte| byte.is_ascii_digit()) && number != "000"
}

fn valid_evidence_id(value: &str) -> bool {
    let mut parts = value.split('-');
    let prefix = parts.next();
    let timestamp = parts.next().unwrap_or_default();
    let sequence = parts.next().unwrap_or_default();
    prefix == Some("EVD")
        && parts.next().is_none()
        && chrono::NaiveDateTime::parse_from_str(timestamp, "%Y%m%dT%H%M%S").is_ok()
        && sequence.len() == 3
        && sequence
            .parse::<u16>()
            .is_ok_and(|number| (1..=999).contains(&number))
}

fn id_character(character: char) -> bool {
    character.is_alphanumeric() || character == '-'
}

fn next_character(text: &str, offset: usize) -> usize {
    offset + text[offset..].chars().next().map_or(1, char::len_utf8)
}

fn goal_fragments(body: &str) -> BTreeSet<String> {
    let mut fragments = BTreeSet::new();
    for (ordinal, (_, item)) in goal_items(body).into_iter().enumerate() {
        fragments.extend(goal_item_fragments(item));
        fragments.insert(format!("sc-{}", ordinal + 1));
    }
    fragments
}

fn goal_item_fragments(item: &str) -> Vec<String> {
    let normalized = criterion_text(item);
    let legacy_hash = format!("sc-{:x}", Sha256::digest(normalized.as_bytes()))[..11].to_owned();
    let mut fragments = vec![legacy_hash];
    if let Some(id) = explicit_goal_id(item) {
        fragments.push(id.to_ascii_lowercase());
    }
    fragments
}

fn plan_fragments(body: &str) -> BTreeSet<String> {
    let mut fragments = BTreeSet::new();
    for id in delivery_map_ids(body) {
        let normalized = id.to_ascii_lowercase();
        fragments.insert(normalized.clone());
        fragments.insert(format!("task-{normalized}"));
    }
    fragments
}

fn goal_items(body: &str) -> Vec<(usize, &str)> {
    let mut in_section = false;
    let mut items = Vec::new();
    for (index, line) in body.lines().enumerate() {
        if let Some(heading) = line.strip_prefix("## ") {
            in_section = heading.trim().eq_ignore_ascii_case("Success Criteria");
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some(item) = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
            .or_else(|| line.strip_prefix("+ "))
        {
            items.push((index + 1, item));
        }
    }
    items
}

pub fn explicit_goal_id(item: &str) -> Option<&str> {
    let remainder = item.strip_prefix('[')?;
    let (id, _) = remainder.split_once(']')?;
    valid_goal_id(id).then_some(id)
}

pub fn criterion_text(item: &str) -> String {
    let item = explicit_goal_id(item)
        .and_then(|id| item.strip_prefix(&format!("[{id}]")))
        .unwrap_or(item);
    item.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn delivery_map_ids(body: &str) -> Vec<String> {
    delivery_map_id_rows(body)
        .into_iter()
        .map(|(_, id)| id)
        .filter(|id| valid_task_id(id) || legacy_task_id(id))
        .collect()
}

fn delivery_map_id_rows(body: &str) -> Vec<(usize, String)> {
    let mut in_section = false;
    let mut id_index = None;
    let mut ids = Vec::new();
    for (line_number, line) in body.lines().enumerate() {
        if let Some(heading) = line.strip_prefix("## ") {
            in_section = heading.trim().eq_ignore_ascii_case("Delivery Map");
            id_index = None;
            continue;
        }
        if !in_section || !line.trim_start().starts_with('|') {
            continue;
        }
        let cells = table_cells(line);
        if id_index.is_none() {
            id_index = cells
                .iter()
                .position(|cell| cell.eq_ignore_ascii_case("ID"));
            continue;
        }
        let Some(index) = id_index else {
            continue;
        };
        let Some(id) = cells.get(index).map(String::as_str) else {
            continue;
        };
        if !id
            .chars()
            .all(|character| character == '-' || character == ':')
        {
            ids.push((line_number + 1, id.to_owned()));
        }
    }
    ids
}

fn legacy_task_id(value: &str) -> bool {
    let Some(number) = value.strip_prefix("T-") else {
        return false;
    };
    !number.is_empty() && number.len() <= 3 && number.bytes().all(|byte| byte.is_ascii_digit())
}

fn table_cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_fragments_include_canonical_and_legacy_anchors() {
        let body = "## Success Criteria\n\n- [SC-001] Stable result\n";
        let fragments = local_fragments(EntryType::Goal, body);
        assert!(fragments.contains("sc-001"));
        assert!(fragments.contains("sc-1"));
        assert!(fragments.iter().any(|fragment| fragment.starts_with("sc-")));
    }

    #[test]
    fn goal_findings_require_unique_fixed_width_ids() {
        let body = "## Success Criteria\n\n- Missing\n- [SC-001] First\n- [SC-001] Duplicate\n";
        let findings = goal_id_findings(body);
        assert_eq!(findings.len(), 2);
        assert!(findings[0].message.contains("must start"));
        assert!(findings[1].message.contains("duplicate"));
    }

    #[test]
    fn nested_goal_details_do_not_define_criteria() {
        let body = "## Success Criteria\n\n- [SC-001] Stable result\n  - explanatory detail\n";
        assert!(goal_id_findings(body).is_empty());
        assert_eq!(fragment_match_count(EntryType::Goal, body, "sc-001"), 1);
    }

    #[test]
    fn plan_fragments_accept_canonical_and_legacy_task_anchors() {
        let body = "## Delivery Map\n\n| ID | State |\n| --- | --- |\n| T-001 | verified |\n";
        let fragments = local_fragments(EntryType::Plan, body);
        assert!(fragments.contains("t-001"));
        assert!(fragments.contains("task-t-001"));
    }

    #[test]
    fn duplicate_local_ids_are_ambiguous_and_do_not_resolve() {
        let goal = "## Success Criteria\n\n- [SC-001] First\n- [SC-001] Duplicate\n";
        let plan = "## Delivery Map\n\n| ID | State |\n| --- | --- |\n| T-001 | verified |\n| T-001 | blocked |\n";
        assert_eq!(fragment_match_count(EntryType::Goal, goal, "sc-001"), 2);
        assert!(!fragment_exists(EntryType::Goal, goal, "sc-001"));
        assert_eq!(fragment_match_count(EntryType::Plan, plan, "t-001"), 2);
        assert!(!fragment_exists(EntryType::Plan, plan, "t-001"));
    }

    #[test]
    fn plan_findings_require_unique_fixed_width_ids() {
        let body = "## Delivery Map\n\n| ID | State |\n| --- | --- |\n| T-1 | verified |\n| T-001 | verified |\n| T-001 | blocked |\n";
        let findings = plan_id_findings(body);
        assert_eq!(findings.len(), 2);
        assert!(findings[0].message.contains("fixed-width"));
        assert!(findings[1].message.contains("duplicate"));
    }

    #[test]
    fn reference_spans_find_entries_fragments_and_evidence() {
        let text = "See GOAL-20260723T120000-001-safe-sync#sc-001 and \
                    EVD-20260723T120500-001; ignore SC-001.";
        let spans = reference_spans(text);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].value, "GOAL-20260723T120000-001-safe-sync#sc-001");
        assert!(!spans[0].evidence);
        assert!(spans[1].evidence);
    }
}
