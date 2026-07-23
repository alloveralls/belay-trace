use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn belay() -> Command {
    Command::new(env!("CARGO_BIN_EXE_belay"))
}

fn run(root: &Path, arguments: &[&str]) -> String {
    let output = belay()
        .args(arguments)
        .current_dir(root)
        .output()
        .expect("run belay command");
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout)
        .expect("command output is UTF-8")
        .trim()
        .to_owned()
}

fn start(root: &Path) -> (Child, u16) {
    let mut child = belay()
        .arg("browse")
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start browse");
    let stdout = child.stdout.take().expect("capture browse stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read Browse URL");
    let port = line
        .trim()
        .strip_prefix("Belay Browse: http://127.0.0.1:")
        .and_then(|value| value.strip_suffix('/'))
        .expect("Browse prints loopback URL")
        .parse()
        .expect("URL contains port");
    (child, port)
}

fn request(port: u16, request: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to Browse");
    stream
        .write_all(request.as_bytes())
        .expect("write HTTP request");
    stream.shutdown(std::net::Shutdown::Write).ok();
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read HTTP response");
    response
}

fn digest_tree(root: &Path) -> Vec<u8> {
    fn visit(root: &Path, path: &Path, hasher: &mut Sha256) {
        let mut entries = fs::read_dir(path)
            .expect("read state directory")
            .map(|entry| entry.expect("read directory entry").path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            if entry
                .extension()
                .is_some_and(|extension| extension == "sqlite-wal" || extension == "sqlite-shm")
            {
                continue;
            }
            if entry.is_dir() {
                visit(root, &entry, hasher);
            } else {
                hasher.update(
                    entry
                        .strip_prefix(root)
                        .expect("relative path")
                        .as_os_str()
                        .as_encoded_bytes(),
                );
                hasher.update(fs::read(entry).expect("read state file"));
            }
        }
    }
    let mut hasher = Sha256::new();
    visit(root, &root.join(".belay/entries"), &mut hasher);
    visit(root, &root.join(".belay/evidence"), &mut hasher);
    hasher.update(fs::read(root.join(".belay/state/belay.sqlite")).expect("read SQLite"));
    hasher.finalize().to_vec()
}

#[test]
fn browse_http_security_atomic_reload_and_repository_invariance() {
    let temporary = tempdir().expect("create temporary repository");
    let root = temporary.path();
    fs::create_dir(root.join(".git")).expect("create repository marker");
    run(root, &["init"]);
    run(
        root,
        &[
            "add",
            "work",
            "--title",
            "HTTP fixture",
            "--body",
            "Searchable HTTP fixture",
        ],
    );
    let before = digest_tree(root);
    let (mut child, port) = start(root);
    let host = format!("127.0.0.1:{port}");
    let get = request(
        port,
        &format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
    );
    assert!(get.starts_with("HTTP/1.1 200"), "{get}");
    assert!(get.contains("Content-Security-Policy: default-src 'none'"));
    assert!(get.contains("X-Content-Type-Options: nosniff"));
    assert!(get.contains("X-Frame-Options: DENY"));
    let nonce = get
        .split("data-reload-nonce=\"")
        .nth(1)
        .and_then(|tail| tail.split('"').next())
        .expect("page contains reload nonce")
        .to_owned();

    let bad_host = request(
        port,
        "GET / HTTP/1.1\r\nHost: attacker.invalid\r\nConnection: close\r\n\r\n",
    );
    assert!(bad_host.starts_with("HTTP/1.1 403"));
    let bad_origin = request(
        port,
        &format!(
            "GET / HTTP/1.1\r\nHost: {host}\r\nOrigin: https://attacker.invalid\r\nConnection: close\r\n\r\n"
        ),
    );
    assert!(bad_origin.starts_with("HTTP/1.1 403"));
    let cross_loopback_origin = request(
        port,
        &format!(
            "GET / HTTP/1.1\r\nHost: localhost:{port}\r\nOrigin: http://127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
        ),
    );
    assert!(cross_loopback_origin.starts_with("HTTP/1.1 403"));
    let bad_nonce = request(
        port,
        &format!(
            "POST /api/reload HTTP/1.1\r\nHost: {host}\r\nOrigin: http://{host}\r\nX-Belay-Nonce: wrong\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ),
    );
    assert!(bad_nonce.starts_with("HTTP/1.1 403"));
    let no_origin = request(
        port,
        &format!(
            "POST /api/reload HTTP/1.1\r\nHost: {host}\r\nX-Belay-Nonce: {nonce}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ),
    );
    assert!(no_origin.starts_with("HTTP/1.1 403"));

    let database = root.join(".belay/state/belay.sqlite");
    let held = root.join(".belay/state/belay.sqlite.held");
    fs::rename(&database, &held).expect("temporarily hide source database");
    let failed_reload = request(
        port,
        &format!(
            "POST /api/reload HTTP/1.1\r\nHost: {host}\r\nOrigin: http://{host}\r\nX-Belay-Nonce: {nonce}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ),
    );
    assert!(failed_reload.starts_with("HTTP/1.1 503"));
    assert!(failed_reload.contains("previous snapshot retained"));
    let retained = request(
        port,
        &format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
    );
    assert!(retained.starts_with("HTTP/1.1 200"));
    assert!(retained.contains("HTTP fixture"));
    fs::rename(&held, &database).expect("restore source database");

    let successful_reload = request(
        port,
        &format!(
            "POST /api/reload HTTP/1.1\r\nHost: {host}\r\nOrigin: http://{host}\r\nX-Belay-Nonce: {nonce}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ),
    );
    assert!(successful_reload.starts_with("HTTP/1.1 200"));
    let reloaded_page = request(
        port,
        &format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
    );
    assert!(reloaded_page.contains("Snapshot reloaded atomically."));
    child.kill().expect("stop Browse server");
    child.wait().expect("wait for Browse server");
    assert_eq!(digest_tree(root), before);
}

#[test]
fn successful_reload_swaps_to_the_new_complete_generation() {
    let temporary = tempdir().expect("create temporary repository");
    let root = temporary.path();
    fs::create_dir(root.join(".git")).expect("create repository marker");
    run(root, &["init"]);
    run(
        root,
        &["add", "note", "--title", "Before reload", "--body", "old"],
    );
    let (mut child, port) = start(root);
    let host = format!("127.0.0.1:{port}");
    let initial = request(
        port,
        &format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
    );
    let nonce = initial
        .split("data-reload-nonce=\"")
        .nth(1)
        .and_then(|tail| tail.split('"').next())
        .expect("page contains reload nonce");
    run(
        root,
        &["add", "note", "--title", "After reload", "--body", "new"],
    );
    let still_old = request(
        port,
        &format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
    );
    assert!(!still_old.contains("After reload"));
    let reload = request(
        port,
        &format!(
            "POST /api/reload HTTP/1.1\r\nHost: {host}\r\nOrigin: http://{host}\r\nX-Belay-Nonce: {nonce}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ),
    );
    assert!(reload.starts_with("HTTP/1.1 200"));
    let updated = request(
        port,
        &format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
    );
    assert!(updated.contains("After reload"));
    child.kill().expect("stop Browse server");
    child.wait().expect("wait for Browse server");
}
