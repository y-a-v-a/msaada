//! Path traversal defense tests.
//!
//! These use a raw TCP client so the attack path reaches the server verbatim —
//! reqwest/hyper normalize `..` segments before sending, which would defeat the
//! test. Each probe asserts that the request does NOT return a 200 with the
//! secret file's contents.

mod common;

use common::prelude::*;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const SECRET_MARKER: &str = "TOP_SECRET_MARKER_8d3f1a";

fn raw_http_get(port: u16, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .ok();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .ok();

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        path, port
    );
    stream.write_all(request.as_bytes()).expect("write");

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).ok();
    let response = String::from_utf8_lossy(&buf).to_string();

    let status = response
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    (status, response)
}

fn assert_traversal_blocked(port: u16, path: &str) {
    let (status, body) = raw_http_get(port, path);
    assert_ne!(
        status, 200,
        "path {:?} unexpectedly returned 200\n---\n{}",
        path, body
    );
    assert!(
        !body.contains(SECRET_MARKER),
        "path {:?} leaked secret file contents\n---\n{}",
        path,
        body
    );
}

#[tokio::test]
async fn rejects_parent_dir_traversal() {
    // Layout:
    //   outer/
    //     secret.txt        <- must NOT be reachable
    //     serve/
    //       index.html
    let outer = tempfile::TempDir::new().expect("tempdir");
    let serve_dir = outer.path().join("serve");
    std::fs::create_dir_all(&serve_dir).unwrap();
    std::fs::write(
        outer.path().join("secret.txt"),
        format!("{}\nshould never leak", SECRET_MARKER),
    )
    .unwrap();
    std::fs::write(serve_dir.join("index.html"), "<html>inside</html>").unwrap();

    let server = TestServer::new_with_options(Some(serve_dir.clone()), None)
        .await
        .expect("start server");

    // Sanity check: the in-root file IS reachable.
    let (status, body) = raw_http_get(server.port, "/index.html");
    assert_eq!(status, 200, "sanity GET failed\n{}", body);
    assert!(body.contains("inside"));

    // Literal `..` segments in the path.
    assert_traversal_blocked(server.port, "/../secret.txt");
    assert_traversal_blocked(server.port, "/../../secret.txt");
    assert_traversal_blocked(server.port, "/foo/../../secret.txt");

    // URL-encoded `..` (%2E%2E). If the server decodes before normalizing,
    // these resolve to `..` and must still be blocked.
    assert_traversal_blocked(server.port, "/%2E%2E/secret.txt");
    assert_traversal_blocked(server.port, "/%2e%2e/secret.txt");
    assert_traversal_blocked(server.port, "/foo/%2E%2E/%2E%2E/secret.txt");

    // Fully-encoded separator.
    assert_traversal_blocked(server.port, "/%2E%2E%2Fsecret.txt");

    // Mixed absolute-style path (the server should reject leading `..` after
    // stripping the leading slash).
    assert_traversal_blocked(server.port, "/..%2fsecret.txt");

    // Keep `outer` alive until after the server is dropped.
    drop(server);
    drop(outer);
}

#[cfg(unix)]
#[tokio::test]
async fn rejects_symlink_escaping_root_by_default() {
    let outer = tempfile::TempDir::new().expect("tempdir");
    let serve_dir = outer.path().join("serve");
    std::fs::create_dir_all(&serve_dir).unwrap();
    std::fs::write(
        outer.path().join("secret.txt"),
        format!("{}\nshould never leak", SECRET_MARKER),
    )
    .unwrap();
    std::fs::write(serve_dir.join("index.html"), "inside").unwrap();

    // serve/escape -> outer/secret.txt  (escapes the served root)
    std::os::unix::fs::symlink(
        outer.path().join("secret.txt"),
        serve_dir.join("escape"),
    )
    .unwrap();

    let server = TestServer::new_with_options(Some(serve_dir.clone()), None)
        .await
        .expect("start server");

    let (status, body) = raw_http_get(server.port, "/escape");
    assert!(
        status == 403 || status == 404,
        "symlink escape expected 403/404, got {}\n{}",
        status,
        body
    );
    assert!(
        !body.contains(SECRET_MARKER),
        "symlink escape leaked secret contents\n{}",
        body
    );

    drop(server);
    drop(outer);
}
