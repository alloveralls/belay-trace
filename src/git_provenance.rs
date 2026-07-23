use std::collections::HashSet;
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct GitLimits {
    pub timeout: Duration,
    pub stderr_bytes: usize,
    pub metadata_bytes: usize,
    pub file_list_bytes: usize,
    pub max_files: usize,
    pub blob_bytes: usize,
    pub diff_bytes: usize,
}

impl Default for GitLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            stderr_bytes: 64 * 1024,
            metadata_bytes: 256 * 1024,
            file_list_bytes: 2 * 1024 * 1024,
            max_files: 500,
            blob_bytes: 1024 * 1024,
            diff_bytes: 512 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum CommitAvailability {
    Available { oid: String },
    Unknown,
    Invalid { reason: String },
    Ambiguous,
    Missing,
    NotCommit { object_type: String },
    GitUnavailable { reason: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitDetail {
    pub evidence_sha: String,
    pub oid: String,
    pub parents: Vec<String>,
    pub comparison_base: String,
    pub is_root: bool,
    pub is_merge: bool,
    pub subject: String,
    pub metadata_truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileList {
    pub commit_oid: String,
    pub comparison_base: String,
    pub is_root: bool,
    pub files: Vec<FileChange>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileKind {
    Regular,
    Symlink,
    Gitlink,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileChange {
    pub opaque_id: String,
    pub status: String,
    pub similarity: Option<u8>,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub path_encoding_lossy: bool,
    pub old_mode: String,
    pub new_mode: String,
    pub old_oid: String,
    pub new_oid: String,
    pub kind: FileKind,
    #[serde(skip)]
    old_path_bytes: Option<Vec<u8>>,
    #[serde(skip)]
    new_path_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileDetail {
    pub commit_oid: String,
    pub comparison_base: String,
    pub change: FileChange,
    pub content: FileContent,
    pub diff: Option<BoundedText>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FileContent {
    Text { text: String, size: u64 },
    Symlink { target: String, size: u64 },
    Gitlink { oid: String },
    Binary { size: u64 },
    NonUtf8 { size: u64 },
    Omitted { size: u64, reason: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundedText {
    pub text: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserCommit {
    pub subject: String,
    pub base_sha: String,
    pub truncated: bool,
    pub files: Vec<BrowserFile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserFile {
    pub status: String,
    pub opaque_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserFileDetail {
    pub path: String,
    pub kind: String,
    pub size: u64,
    pub truncated: bool,
    pub content: Option<String>,
    pub diff: Option<String>,
}

#[derive(Debug, Error)]
pub enum GitProvenanceError {
    #[error("commit is not referenced by evidence in this snapshot")]
    NotAllowed,
    #[error("commit is unavailable: {0}")]
    Unavailable(String),
    #[error("Git command failed: {0}")]
    Git(String),
    #[error("Git output was invalid: {0}")]
    InvalidOutput(String),
    #[error("Git command timed out")]
    Timeout,
    #[error("file was not listed for this commit")]
    FileNotFound,
}

pub struct GitProvenanceReader {
    root: PathBuf,
    limits: GitLimits,
    allowed: HashSet<String>,
}

pub type GitReader = GitProvenanceReader;

pub fn read_head(root: &Path) -> Result<String, GitProvenanceError> {
    let reader = GitProvenanceReader::new(root, std::iter::empty());
    let output = reader.git(
        &[os("rev-parse"), os("--verify"), os("HEAD^{commit}")],
        None,
        129,
    )?;
    require_success(&output)?;
    if output.stdout.truncated {
        return Err(GitProvenanceError::InvalidOutput(
            "HEAD object ID exceeded its output limit".to_owned(),
        ));
    }
    let head = std::str::from_utf8(&output.stdout.bytes)
        .map_err(|_| GitProvenanceError::InvalidOutput("HEAD object ID is not UTF-8".to_owned()))?
        .trim();
    if head.is_empty() || !head.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GitProvenanceError::InvalidOutput(
            "HEAD object ID is not hexadecimal".to_owned(),
        ));
    }
    Ok(head.to_owned())
}

impl GitProvenanceReader {
    pub fn new(root: impl Into<PathBuf>, evidence_shas: impl IntoIterator<Item = String>) -> Self {
        Self::with_limits(root, evidence_shas, GitLimits::default())
    }

    pub fn with_limits(
        root: impl Into<PathBuf>,
        evidence_shas: impl IntoIterator<Item = String>,
        limits: GitLimits,
    ) -> Self {
        Self {
            root: root.into(),
            limits,
            allowed: evidence_shas.into_iter().collect(),
        }
    }

    pub fn availability(&self, requested: &str) -> Option<CommitAvailability> {
        self.allowed
            .contains(requested)
            .then(|| self.resolve_evidence_sha(requested))
    }

    pub fn commits_behind(&self, requested: &str, head: &str) -> Result<u32, GitProvenanceError> {
        if head.is_empty() || !head.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(GitProvenanceError::InvalidOutput(
                "snapshot HEAD is not a hexadecimal object ID".to_owned(),
            ));
        }
        if !self.allowed.contains(requested) {
            return Err(GitProvenanceError::NotAllowed);
        }
        let availability = self.resolve_evidence_sha(requested);
        let CommitAvailability::Available { oid } = &availability else {
            return Err(GitProvenanceError::Unavailable(availability_reason(
                &availability,
            )));
        };
        let range = format!("{oid}..{head}");
        let output = self.git(&[os("rev-list"), os("--count"), os(&range)], None, 32)?;
        require_success(&output)?;
        if output.stdout.truncated {
            return Err(GitProvenanceError::InvalidOutput(
                "rev-list count exceeded its output limit".to_owned(),
            ));
        }
        std::str::from_utf8(&output.stdout.bytes)
            .map_err(|_| {
                GitProvenanceError::InvalidOutput("rev-list count is not UTF-8".to_owned())
            })?
            .trim()
            .parse::<u32>()
            .map_err(|_| GitProvenanceError::InvalidOutput("rev-list count is invalid".to_owned()))
    }

    pub fn commit(&self, requested: &str) -> Result<BrowserCommit, GitProvenanceError> {
        let metadata = self.commit_metadata(requested)?;
        let list = self.files_for_commit(&metadata)?;
        Ok(BrowserCommit {
            subject: metadata.subject,
            base_sha: metadata.comparison_base,
            truncated: metadata.metadata_truncated || list.truncated,
            files: list
                .files
                .into_iter()
                .map(|file| BrowserFile {
                    status: file.status,
                    opaque_id: file.opaque_id,
                    path: file
                        .new_path
                        .or(file.old_path)
                        .unwrap_or_else(|| "(unknown path)".to_owned()),
                })
                .collect(),
        })
    }

    pub fn commit_metadata(&self, requested: &str) -> Result<CommitDetail, GitProvenanceError> {
        if !self.allowed.contains(requested) {
            return Err(GitProvenanceError::NotAllowed);
        }
        let availability = self.resolve_evidence_sha(requested);
        let CommitAvailability::Available { oid } = availability else {
            return Err(GitProvenanceError::Unavailable(availability_reason(
                &availability,
            )));
        };
        let output = self.git(
            &[os("cat-file"), os("-p"), os(&oid)],
            None,
            self.limits.metadata_bytes,
        )?;
        require_success(&output)?;
        let (parents, subject) = parse_commit(&output.stdout.bytes)?;
        let (comparison_base, is_root) = if let Some(parent) = parents.first() {
            (parent.clone(), false)
        } else {
            (self.empty_tree_oid()?, true)
        };
        Ok(CommitDetail {
            evidence_sha: requested.to_owned(),
            oid,
            is_merge: parents.len() > 1,
            parents,
            comparison_base,
            is_root,
            subject,
            metadata_truncated: output.stdout.truncated,
        })
    }

    pub fn files(&self, requested: &str) -> Result<FileList, GitProvenanceError> {
        let commit = self.commit_metadata(requested)?;
        self.files_for_commit(&commit)
    }

    fn files_for_commit(&self, commit: &CommitDetail) -> Result<FileList, GitProvenanceError> {
        let mut arguments = vec![
            os("diff-tree"),
            os("-r"),
            os("-M"),
            os("-l1000"),
            os("--no-commit-id"),
            os("--raw"),
            os("-z"),
            os("--no-abbrev"),
        ];
        if commit.is_root {
            arguments.push(os("--root"));
            arguments.push(os(&commit.oid));
        } else {
            arguments.push(os(&commit.comparison_base));
            arguments.push(os(&commit.oid));
        }
        let output = self.git(&arguments, None, self.limits.file_list_bytes)?;
        require_success(&output)?;
        let (mut files, incomplete) = parse_raw_diff(
            &output.stdout.bytes,
            &commit.oid,
            &commit.comparison_base,
            self.limits.max_files,
        )?;
        let too_many = files.len() > self.limits.max_files;
        files.truncate(self.limits.max_files);
        Ok(FileList {
            commit_oid: commit.oid.clone(),
            comparison_base: commit.comparison_base.clone(),
            is_root: commit.is_root,
            files,
            truncated: output.stdout.truncated || incomplete || too_many,
        })
    }

    pub fn file(
        &self,
        requested: &str,
        opaque_id: &str,
    ) -> Result<BrowserFileDetail, GitProvenanceError> {
        let detail = self.file_detail(requested, opaque_id)?;
        let path = detail
            .change
            .new_path
            .clone()
            .or_else(|| detail.change.old_path.clone())
            .unwrap_or_else(|| "(unknown path)".to_owned());
        let (kind, size, content, content_truncated) = match detail.content {
            FileContent::Text { text, size } => ("text", size, Some(text), false),
            FileContent::Symlink { target, size } => ("symlink", size, Some(target), false),
            FileContent::Gitlink { oid } => ("gitlink", 0, Some(oid), false),
            FileContent::Binary { size } => ("binary", size, None, false),
            FileContent::NonUtf8 { size } => ("non-UTF-8", size, None, false),
            FileContent::Omitted { size, .. } => ("omitted", size, None, true),
        };
        let diff_truncated = detail.diff.as_ref().is_some_and(|diff| diff.truncated);
        Ok(BrowserFileDetail {
            path,
            kind: kind.to_owned(),
            size,
            truncated: content_truncated || diff_truncated,
            content,
            diff: detail.diff.map(|diff| diff.text),
        })
    }

    pub fn file_detail(
        &self,
        requested: &str,
        opaque_id: &str,
    ) -> Result<FileDetail, GitProvenanceError> {
        let list = self.files(requested)?;
        let change = list
            .files
            .iter()
            .find(|change| change.opaque_id == opaque_id)
            .cloned()
            .ok_or(GitProvenanceError::FileNotFound)?;
        let (oid, mode) = if change.status.starts_with('D') {
            (&change.old_oid, &change.old_mode)
        } else {
            (&change.new_oid, &change.new_mode)
        };
        let content = if mode == "160000" {
            FileContent::Gitlink { oid: oid.clone() }
        } else {
            self.blob_content(oid, mode == "120000")?
        };
        let diff = if matches!(
            content,
            FileContent::Binary { .. }
                | FileContent::NonUtf8 { .. }
                | FileContent::Omitted { .. }
                | FileContent::Gitlink { .. }
        ) {
            None
        } else {
            Some(self.file_diff(
                &list.comparison_base,
                &list.commit_oid,
                list.is_root,
                &change,
            )?)
        };
        Ok(FileDetail {
            commit_oid: list.commit_oid,
            comparison_base: list.comparison_base,
            change,
            content,
            diff,
        })
    }

    fn resolve_evidence_sha(&self, sha: &str) -> CommitAvailability {
        if sha == "unknown" {
            return CommitAvailability::Unknown;
        }
        if sha.len() < 4 || !sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return CommitAvailability::Invalid {
                reason: "commit must be an ASCII hexadecimal object ID or prefix".to_owned(),
            };
        }
        let argument = format!("--disambiguate={sha}");
        let output = match self.git(&[os("rev-parse"), os(&argument)], None, 4096) {
            Ok(output) => output,
            Err(error) => {
                return CommitAvailability::GitUnavailable {
                    reason: error.to_string(),
                };
            }
        };
        if !output.status.success() {
            return CommitAvailability::GitUnavailable {
                reason: stderr_message(&output),
            };
        }
        if output.stdout.truncated {
            return CommitAvailability::Ambiguous;
        }
        let candidates = String::from_utf8_lossy(&output.stdout.bytes)
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return CommitAvailability::Missing;
        }
        if candidates.len() != 1 {
            return CommitAvailability::Ambiguous;
        }
        let oid = &candidates[0];
        let kind = match self.git(&[os("cat-file"), os("-t"), os(oid)], None, 128) {
            Ok(output) if output.status.success() && !output.stdout.truncated => {
                String::from_utf8_lossy(&output.stdout.bytes)
                    .trim()
                    .to_owned()
            }
            Ok(output) => {
                return CommitAvailability::GitUnavailable {
                    reason: stderr_message(&output),
                };
            }
            Err(error) => {
                return CommitAvailability::GitUnavailable {
                    reason: error.to_string(),
                };
            }
        };
        if kind != "commit" {
            return CommitAvailability::NotCommit { object_type: kind };
        }
        CommitAvailability::Available { oid: oid.clone() }
    }

    fn empty_tree_oid(&self) -> Result<String, GitProvenanceError> {
        let output = self.git(
            &[os("hash-object"), os("-t"), os("tree"), os("--stdin")],
            Some(&[]),
            128,
        )?;
        require_success(&output)?;
        Ok(String::from_utf8_lossy(&output.stdout.bytes)
            .trim()
            .to_owned())
    }

    fn blob_content(&self, oid: &str, symlink: bool) -> Result<FileContent, GitProvenanceError> {
        let size_output = self.git(&[os("cat-file"), os("-s"), os(oid)], None, 128)?;
        require_success(&size_output)?;
        let size = String::from_utf8_lossy(&size_output.stdout.bytes)
            .trim()
            .parse::<u64>()
            .map_err(|_| GitProvenanceError::InvalidOutput("invalid blob size".to_owned()))?;
        if size > self.limits.blob_bytes as u64 {
            return Ok(FileContent::Omitted {
                size,
                reason: format!("content exceeds {} byte limit", self.limits.blob_bytes),
            });
        }
        let output = self.git(
            &[os("cat-file"), os("blob"), os(oid)],
            None,
            self.limits.blob_bytes,
        )?;
        require_success(&output)?;
        if output.stdout.truncated {
            return Ok(FileContent::Omitted {
                size,
                reason: "content was truncated by the Git output limit".to_owned(),
            });
        }
        if output.stdout.bytes.contains(&0) {
            return Ok(FileContent::Binary { size });
        }
        let Ok(text) = String::from_utf8(output.stdout.bytes) else {
            return Ok(FileContent::NonUtf8 { size });
        };
        if symlink {
            Ok(FileContent::Symlink { target: text, size })
        } else {
            Ok(FileContent::Text { text, size })
        }
    }

    fn file_diff(
        &self,
        base: &str,
        commit: &str,
        is_root: bool,
        change: &FileChange,
    ) -> Result<BoundedText, GitProvenanceError> {
        let mut args = vec![
            os("diff-tree"),
            os("-p"),
            os("-M"),
            os("-l1000"),
            os("--no-commit-id"),
            os("--no-ext-diff"),
            os("--no-textconv"),
        ];
        if is_root {
            args.push(os("--root"));
            args.push(os(commit));
        } else {
            args.push(os(base));
            args.push(os(commit));
        }
        args.push(os("--"));
        if let Some(path) = &change.old_path_bytes {
            args.push(path_arg(path)?);
        }
        if change.new_path_bytes != change.old_path_bytes {
            if let Some(path) = &change.new_path_bytes {
                args.push(path_arg(path)?);
            }
        }
        let output = self.git(&args, None, self.limits.diff_bytes)?;
        require_success(&output)?;
        Ok(BoundedText {
            text: String::from_utf8_lossy(&output.stdout.bytes).into_owned(),
            truncated: output.stdout.truncated,
        })
    }

    fn git(
        &self,
        args: &[OsString],
        stdin: Option<&[u8]>,
        stdout_limit: usize,
    ) -> Result<ProcessOutput, GitProvenanceError> {
        run_git(
            &self.root,
            args,
            stdin,
            stdout_limit,
            self.limits.stderr_bytes,
            self.limits.timeout,
        )
    }
}

fn os(value: &str) -> OsString {
    OsString::from(value)
}

#[cfg(unix)]
fn path_arg(bytes: &[u8]) -> Result<OsString, GitProvenanceError> {
    use std::os::unix::ffi::OsStringExt;
    Ok(OsString::from_vec(bytes.to_vec()))
}

#[cfg(not(unix))]
fn path_arg(bytes: &[u8]) -> Result<OsString, GitProvenanceError> {
    String::from_utf8(bytes.to_vec())
        .map(OsString::from)
        .map_err(|_| {
            GitProvenanceError::Unavailable(
                "non-UTF-8 Git path is unsupported on this platform".to_owned(),
            )
        })
}

fn availability_reason(availability: &CommitAvailability) -> String {
    match availability {
        CommitAvailability::Available { .. } => "available".to_owned(),
        CommitAvailability::Unknown => "commit is unknown".to_owned(),
        CommitAvailability::Invalid { reason } => reason.clone(),
        CommitAvailability::Ambiguous => "commit prefix is ambiguous".to_owned(),
        CommitAvailability::Missing => "commit object is missing".to_owned(),
        CommitAvailability::NotCommit { object_type } => {
            format!("object is {object_type}, not a commit")
        }
        CommitAvailability::GitUnavailable { reason } => format!("Git is unavailable: {reason}"),
    }
}

fn parse_commit(bytes: &[u8]) -> Result<(Vec<String>, String), GitProvenanceError> {
    let separator = bytes
        .windows(2)
        .position(|pair| pair == b"\n\n")
        .ok_or_else(|| {
            GitProvenanceError::InvalidOutput("commit headers are incomplete".to_owned())
        })?;
    let headers = String::from_utf8_lossy(&bytes[..separator]);
    let parents = headers
        .lines()
        .filter_map(|line| line.strip_prefix("parent ").map(str::to_owned))
        .collect();
    let message = String::from_utf8_lossy(&bytes[separator + 2..]);
    let subject = message.lines().next().unwrap_or_default().to_owned();
    Ok((parents, subject))
}

fn parse_raw_diff(
    bytes: &[u8],
    commit: &str,
    base: &str,
    max_files: usize,
) -> Result<(Vec<FileChange>, bool), GitProvenanceError> {
    let fields = bytes.split(|byte| *byte == 0).collect::<Vec<_>>();
    let mut index = 0;
    let mut files = Vec::new();
    let mut incomplete = false;
    while index < fields.len() && !fields[index].is_empty() {
        let header = std::str::from_utf8(fields[index]).map_err(|_| {
            GitProvenanceError::InvalidOutput("non-ASCII raw diff header".to_owned())
        })?;
        index += 1;
        let parts = header.split_ascii_whitespace().collect::<Vec<_>>();
        if parts.len() != 5 || !parts[0].starts_with(':') {
            return Err(GitProvenanceError::InvalidOutput(
                "malformed raw diff record".to_owned(),
            ));
        }
        if index >= fields.len() {
            incomplete = true;
            break;
        }
        let old_path = fields[index].to_vec();
        index += 1;
        let status = parts[4].to_owned();
        let new_path = if status.starts_with('R') || status.starts_with('C') {
            if index >= fields.len() {
                incomplete = true;
                break;
            }
            let path = fields[index].to_vec();
            index += 1;
            path
        } else {
            old_path.clone()
        };
        if files.len() > max_files {
            return Ok((files, true));
        }
        let old_mode = parts[0].trim_start_matches(':').to_owned();
        let new_mode = parts[1].to_owned();
        let old_oid = parts[2].to_owned();
        let new_oid = parts[3].to_owned();
        let mode = if status.starts_with('D') {
            &old_mode
        } else {
            &new_mode
        };
        let kind = match mode.as_str() {
            "120000" => FileKind::Symlink,
            "160000" => FileKind::Gitlink,
            _ => FileKind::Regular,
        };
        let similarity = status
            .get(1..)
            .filter(|value| !value.is_empty())
            .and_then(|value| value.parse().ok());
        let old_visible = !status.starts_with('A');
        let new_visible = !status.starts_with('D');
        let (old_display, old_lossy) = display_path(&old_path);
        let (new_display, new_lossy) = display_path(&new_path);
        files.push(FileChange {
            opaque_id: opaque_id(
                commit, base, &status, &old_path, &new_path, &old_oid, &new_oid,
            ),
            status,
            similarity,
            old_path: old_visible.then_some(old_display),
            new_path: new_visible.then_some(new_display),
            path_encoding_lossy: old_lossy || new_lossy,
            old_mode,
            new_mode,
            old_oid,
            new_oid,
            old_path_bytes: old_visible.then_some(old_path),
            new_path_bytes: new_visible.then_some(new_path),
            kind,
        });
    }
    Ok((files, incomplete))
}

fn display_path(bytes: &[u8]) -> (String, bool) {
    match std::str::from_utf8(bytes) {
        Ok(path) => (path.to_owned(), false),
        Err(_) => (String::from_utf8_lossy(bytes).into_owned(), true),
    }
}

fn opaque_id(
    commit: &str,
    base: &str,
    status: &str,
    old_path: &[u8],
    new_path: &[u8],
    old_oid: &str,
    new_oid: &str,
) -> String {
    let mut hash = Sha256::new();
    for value in [
        commit.as_bytes(),
        base.as_bytes(),
        status.as_bytes(),
        old_path,
        new_path,
        old_oid.as_bytes(),
        new_oid.as_bytes(),
    ] {
        hash.update(value);
        hash.update([0]);
    }
    format!("{:x}", hash.finalize())
}

struct Captured {
    bytes: Vec<u8>,
    truncated: bool,
}

struct ProcessOutput {
    status: ExitStatus,
    stdout: Captured,
    stderr: Captured,
}

const UNTRUSTED_GIT_ENVIRONMENT: &[&str] = &[
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CEILING_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_CONFIG",
    "GIT_CONFIG_COUNT",
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_SYSTEM",
    "GIT_DIFF_OPTS",
    "GIT_DIR",
    "GIT_DISCOVERY_ACROSS_FILESYSTEM",
    "GIT_EXEC_PATH",
    "GIT_EXTERNAL_DIFF",
    "GIT_GLOB_PATHSPECS",
    "GIT_GRAFT_FILE",
    "GIT_ICASE_PATHSPECS",
    "GIT_INDEX_FILE",
    "GIT_LITERAL_PATHSPECS",
    "GIT_NAMESPACE",
    "GIT_NOGLOB_PATHSPECS",
    "GIT_OBJECT_DIRECTORY",
    "GIT_REPLACE_REF_BASE",
    "GIT_SHALLOW_FILE",
    "GIT_TRACE",
    "GIT_TRACE2",
    "GIT_TRACE2_EVENT",
    "GIT_TRACE2_PERF",
    "GIT_TRACE_PACK_ACCESS",
    "GIT_TRACE_PACKET",
    "GIT_TRACE_PERFORMANCE",
    "GIT_TRACE_SETUP",
    "GIT_TRACE_SHALLOW",
    "GIT_WORK_TREE",
];

fn remove_untrusted_git_environment(command: &mut Command) {
    for variable in UNTRUSTED_GIT_ENVIRONMENT {
        command.env_remove(*variable);
    }
}

fn capture(mut reader: impl Read, limit: usize) -> std::io::Result<Captured> {
    let mut kept = Vec::with_capacity(limit.min(8192));
    let mut buffer = [0_u8; 8192];
    let mut truncated = false;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let remaining = limit.saturating_sub(kept.len());
        let copied = count.min(remaining);
        kept.extend_from_slice(&buffer[..copied]);
        truncated |= copied < count;
    }
    Ok(Captured {
        bytes: kept,
        truncated,
    })
}

fn run_git(
    root: &Path,
    args: &[OsString],
    stdin: Option<&[u8]>,
    stdout_limit: usize,
    stderr_limit: usize,
    timeout: Duration,
) -> Result<ProcessOutput, GitProvenanceError> {
    let mut command = Command::new("git");
    command
        .arg("--no-pager")
        .arg("--no-replace-objects")
        .args(args.iter().map(OsString::as_os_str))
        .current_dir(root)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_PAGER", "cat")
        .env("PAGER", "cat")
        .env("LC_ALL", "C");
    remove_untrusted_git_environment(&mut command);
    let mut child = command
        .spawn()
        .map_err(|error| GitProvenanceError::Git(error.to_string()))?;
    if let Some(input) = stdin {
        let mut child_stdin = child
            .stdin
            .take()
            .ok_or_else(|| GitProvenanceError::Git("Git stdin unavailable".to_owned()))?;
        child_stdin
            .write_all(input)
            .map_err(|error| GitProvenanceError::Git(error.to_string()))?;
    }
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| GitProvenanceError::Git("Git stdout unavailable".to_owned()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| GitProvenanceError::Git("Git stderr unavailable".to_owned()))?;
    let stdout_thread = thread::spawn(move || capture(stdout, stdout_limit));
    let stderr_thread = thread::spawn(move || capture(stderr, stderr_limit));
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| GitProvenanceError::Git(error.to_string()))?
        {
            break status;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            return Err(GitProvenanceError::Timeout);
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout_thread
        .join()
        .map_err(|_| GitProvenanceError::Git("Git stdout reader panicked".to_owned()))?
        .map_err(|error| GitProvenanceError::Git(error.to_string()))?;
    let stderr = stderr_thread
        .join()
        .map_err(|_| GitProvenanceError::Git("Git stderr reader panicked".to_owned()))?
        .map_err(|error| GitProvenanceError::Git(error.to_string()))?;
    Ok(ProcessOutput {
        status,
        stdout,
        stderr,
    })
}

fn require_success(output: &ProcessOutput) -> Result<(), GitProvenanceError> {
    if output.status.success() {
        Ok(())
    } else {
        Err(GitProvenanceError::Git(stderr_message(output)))
    }
}

fn stderr_message(output: &ProcessOutput) -> String {
    let message = String::from_utf8_lossy(&output.stderr.bytes)
        .trim()
        .to_owned();
    if message.is_empty() {
        format!("Git exited with {}", output.status)
    } else if output.stderr.truncated {
        format!("{message} (stderr truncated)")
    } else {
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    struct Fixture {
        temp: TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().expect("temp repo");
            git(temp.path(), &["init", "-q", "-b", "main"]);
            git(temp.path(), &["config", "user.name", "Belay Test"]);
            git(
                temp.path(),
                &["config", "user.email", "belay@example.invalid"],
            );
            Self { temp }
        }
        fn path(&self) -> &Path {
            self.temp.path()
        }
        fn write(&self, path: &str, bytes: &[u8]) {
            let full = self.path().join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(full, bytes).unwrap();
        }
        fn commit_all(&self, message: &str) -> String {
            git(self.path(), &["add", "-A"]);
            git(self.path(), &["commit", "-q", "-m", message]);
            output(self.path(), &["rev-parse", "HEAD"])
        }
    }

    fn git(root: &Path, args: &[&str]) {
        let result = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git");
        assert!(
            result.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    fn output(root: &Path, args: &[&str]) -> String {
        let result = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git");
        assert!(
            result.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8(result.stdout).unwrap().trim().to_owned()
    }

    #[test]
    fn root_normal_rename_delete_and_special_contents_are_bounded() {
        let fixture = Fixture::new();
        fixture.write("plain.txt", b"one\n");
        fixture.write("delete.txt", b"before deletion\n");
        fixture.write("binary.dat", b"a\0b");
        fixture.write("invalid.txt", &[0xff, b'\n']);
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("plain.txt", fixture.path().join("link")).unwrap();
        }
        let root = fixture.commit_all("root subject");
        let reader = GitProvenanceReader::new(fixture.path(), [root.clone()]);
        let detail = reader.commit_metadata(&root).unwrap();
        assert!(detail.is_root);
        assert_eq!(detail.subject, "root subject");
        let root_files = reader.files(&root).unwrap();
        assert!(
            root_files
                .files
                .iter()
                .any(|file| file.new_path.as_deref() == Some("plain.txt"))
        );

        git(fixture.path(), &["mv", "plain.txt", "renamed.txt"]);
        fs::remove_file(fixture.path().join("delete.txt")).unwrap();
        fixture.write("renamed.txt", b"one\ntwo\n");
        let normal = fixture.commit_all("normal");
        let reader = GitProvenanceReader::new(fixture.path(), [normal.clone()]);
        let files = reader.files(&normal).unwrap();
        let renamed = files
            .files
            .iter()
            .find(|file| file.status.starts_with('R'))
            .unwrap();
        assert_eq!(renamed.old_path.as_deref(), Some("plain.txt"));
        assert_eq!(renamed.new_path.as_deref(), Some("renamed.txt"));
        let deleted = files.files.iter().find(|file| file.status == "D").unwrap();
        let deleted_detail = reader.file_detail(&normal, &deleted.opaque_id).unwrap();
        assert!(
            matches!(deleted_detail.content, FileContent::Text { ref text, .. } if text == "before deletion\n")
        );

        let root_reader = GitProvenanceReader::new(fixture.path(), [root.clone()]);
        let root_files = root_reader.files(&root).unwrap();
        for (path, expected) in [("binary.dat", "binary"), ("invalid.txt", "non-utf8")] {
            let file = root_files
                .files
                .iter()
                .find(|file| file.new_path.as_deref() == Some(path))
                .unwrap();
            let detail = root_reader.file_detail(&root, &file.opaque_id).unwrap();
            match (expected, detail.content) {
                ("binary", FileContent::Binary { .. })
                | ("non-utf8", FileContent::NonUtf8 { .. }) => {}
                (_, content) => panic!("unexpected {content:?}"),
            }
            assert!(detail.diff.is_none());
        }
        #[cfg(unix)]
        {
            let link = root_files
                .files
                .iter()
                .find(|file| file.new_path.as_deref() == Some("link"))
                .unwrap();
            let detail = root_reader.file_detail(&root, &link.opaque_id).unwrap();
            assert!(
                matches!(detail.content, FileContent::Symlink { ref target, .. } if target == "plain.txt")
            );
        }
    }

    #[test]
    fn merge_uses_first_parent_and_gitlink_is_metadata_only() {
        let fixture = Fixture::new();
        fixture.write("base.txt", b"base\n");
        let base = fixture.commit_all("base");
        git(fixture.path(), &["checkout", "-q", "-b", "side"]);
        fixture.write("side.txt", b"side\n");
        fixture.commit_all("side");
        git(fixture.path(), &["checkout", "-q", "main"]);
        fixture.write("main.txt", b"main\n");
        let first_parent = fixture.commit_all("main");
        git(
            fixture.path(),
            &["merge", "-q", "--no-ff", "side", "-m", "merge"],
        );
        let merge = output(fixture.path(), &["rev-parse", "HEAD"]);
        let reader = GitProvenanceReader::new(fixture.path(), [merge.clone()]);
        let detail = reader.commit_metadata(&merge).unwrap();
        assert!(detail.is_merge);
        assert_eq!(detail.comparison_base, first_parent);
        assert_ne!(detail.comparison_base, base);
        assert!(
            reader
                .files(&merge)
                .unwrap()
                .files
                .iter()
                .any(|file| file.new_path.as_deref() == Some("side.txt"))
        );

        git(
            fixture.path(),
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("160000,{base},vendor/sub"),
            ],
        );
        git(fixture.path(), &["commit", "-q", "-m", "gitlink"]);
        let gitlink_commit = output(fixture.path(), &["rev-parse", "HEAD"]);
        let reader = GitProvenanceReader::new(fixture.path(), [gitlink_commit.clone()]);
        let file = reader
            .files(&gitlink_commit)
            .unwrap()
            .files
            .into_iter()
            .find(|file| file.new_path.as_deref() == Some("vendor/sub"))
            .unwrap();
        let detail = reader
            .file_detail(&gitlink_commit, &file.opaque_id)
            .unwrap();
        assert!(matches!(detail.content, FileContent::Gitlink { oid } if oid == base));
        assert!(detail.diff.is_none());
    }

    #[test]
    fn only_evidence_commits_are_allowed_and_unavailable_values_are_explicit() {
        let fixture = Fixture::new();
        fixture.write("a", b"a");
        let allowed = fixture.commit_all("allowed");
        fixture.write("b", b"b");
        let denied = fixture.commit_all("denied");
        let missing = "f".repeat(40);
        let reader = GitProvenanceReader::new(
            fixture.path(),
            [
                allowed.clone(),
                "unknown".to_owned(),
                "not-a-sha".to_owned(),
                missing.clone(),
            ],
        );
        assert!(reader.commit(&allowed).is_ok());
        assert!(matches!(
            reader.commit(&denied),
            Err(GitProvenanceError::NotAllowed)
        ));
        assert_eq!(
            reader.availability("unknown"),
            Some(CommitAvailability::Unknown)
        );
        assert!(matches!(
            reader.availability("not-a-sha"),
            Some(CommitAvailability::Invalid { .. })
        ));
        assert_eq!(
            reader.availability(&missing),
            Some(CommitAvailability::Missing)
        );
        assert!(matches!(
            reader.file_detail(&allowed, "../../etc/passwd"),
            Err(GitProvenanceError::FileNotFound)
        ));
    }

    #[test]
    fn oversized_content_and_diff_are_reported_not_silently_buffered() {
        let fixture = Fixture::new();
        fixture.write("large.txt", b"small\n");
        fixture.commit_all("small");
        fixture.write("large.txt", &vec![b'x'; 4096]);
        let commit = fixture.commit_all("large");
        let limits = GitLimits {
            blob_bytes: 8192,
            diff_bytes: 128,
            ..GitLimits::default()
        };
        let reader = GitProvenanceReader::with_limits(fixture.path(), [commit.clone()], limits);
        let file = reader
            .files(&commit)
            .unwrap()
            .files
            .into_iter()
            .next()
            .unwrap();
        let detail = reader.file_detail(&commit, &file.opaque_id).unwrap();
        assert!(matches!(
            detail.content,
            FileContent::Text { size: 4096, .. }
        ));
        assert!(detail.diff.is_some_and(|diff| diff.truncated));

        let reader = GitProvenanceReader::with_limits(
            fixture.path(),
            [commit.clone()],
            GitLimits {
                blob_bytes: 128,
                ..GitLimits::default()
            },
        );
        let file = reader.files(&commit).unwrap().files.remove(0);
        let detail = reader.file_detail(&commit, &file.opaque_id).unwrap();
        assert!(matches!(
            detail.content,
            FileContent::Omitted { size: 4096, .. }
        ));
    }

    #[test]
    fn git_commands_explicitly_remove_repository_and_behavior_overrides() {
        let mut command = Command::new("git");
        for variable in UNTRUSTED_GIT_ENVIRONMENT {
            command.env(*variable, "inherited-value");
        }
        remove_untrusted_git_environment(&mut command);
        let environment = command.get_envs().collect::<Vec<_>>();
        for variable in UNTRUSTED_GIT_ENVIRONMENT {
            assert!(
                environment.iter().any(|(key, value)| {
                    *key == std::ffi::OsStr::new(*variable) && value.is_none()
                }),
                "{variable} was not explicitly removed"
            );
        }
    }
}
