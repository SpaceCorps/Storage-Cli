//! Integration tests for storage-cli against an in-process TCP mock HTTP server.
//!
//! Every test runs with an isolated config directory and mock endpoint,
//! ensuring zero network activity and deterministic assertions.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::Value;

#[derive(Clone, Debug)]
struct Recorded {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

struct Mock {
    url: String,
    log: Arc<Mutex<Vec<Recorded>>>,
}

impl Mock {
    fn start(status_code: u16) -> Mock {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}/");
        let log = Arc::new(Mutex::new(Vec::new()));
        let log_clone = log.clone();

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let log = log_clone.clone();

                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let mut parts = line.split_whitespace();
                    let method = parts.next().unwrap_or("").to_string();
                    let path = parts.next().unwrap_or("").to_string();

                    let mut headers = Vec::new();
                    let mut content_length = 0usize;

                    loop {
                        let mut h = String::new();
                        if reader.read_line(&mut h).unwrap_or(0) == 0 {
                            break;
                        }
                        let trimmed = h.trim_end();
                        if trimmed.is_empty() {
                            break;
                        }
                        if let Some((k, v)) = trimmed.split_once(':') {
                            let (k, v) = (k.trim().to_lowercase(), v.trim().to_string());
                            if k == "content-length" {
                                content_length = v.parse().unwrap_or(0);
                            }
                            headers.push((k, v));
                        }
                    }

                    let mut body = vec![0u8; content_length];
                    if content_length > 0 {
                        let _ = reader.read_exact(&mut body);
                    }

                    log.lock().unwrap().push(Recorded { method: method.clone(), path: path.clone(), headers, body });

                    let status_line = match status_code {
                        201 => "HTTP/1.1 201 Created\r\n",
                        200 => "HTTP/1.1 200 OK\r\n",
                        401 => "HTTP/1.1 401 Unauthorized\r\n",
                        404 => "HTTP/1.1 404 Not Found\r\n",
                        _ => "HTTP/1.1 500 Internal Server Error\r\n",
                    };

                    let body_resp =
                        if status_code >= 400 { r#"{"code":"MockError","message":"mock failure"}"# } else { "" };

                    let response = format!(
                        "{}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        status_line,
                        body_resp.len(),
                        body_resp
                    );
                    let _ = stream.write_all(response.as_bytes());
                });
            }
        });

        Mock { url, log }
    }

    fn requests(&self) -> Vec<Recorded> {
        self.log.lock().unwrap().clone()
    }
}

struct TestEnv {
    dir: PathBuf,
    mock_url: String,
}

impl TestEnv {
    fn new(mock: &Mock) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("storage-test-{}-{}", std::process::id(), id));
        let _ = fs::create_dir_all(&dir);
        TestEnv { dir, mock_url: mock.url.clone() }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_storage"))
            .args(args)
            .env("STORAGE_CONFIG_DIR", &self.dir)
            .env("STORAGE_SECRET_STORE", "plaintext")
            .env("STORAGE_ALLOW_PLAINTEXT_STORE", "1")
            .env("STORAGE_ENDPOINT", &self.mock_url)
            .env("STORAGE_SAS_TOKEN", "sp=racw&se=2026-12-31&sig=mocksig")
            .output()
            .expect("failed to execute binary")
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn test_agent_readme() {
    let mock = Mock::start(200);
    let env = TestEnv::new(&mock);

    let out = env.run(&["agent-readme"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("storage - agent operating manual"));

    let out_json = env.run(&["agent-readme", "--json"]);
    assert!(out_json.status.success());
    let val: Value = serde_json::from_slice(&out_json.stdout).unwrap();
    assert_eq!(val["tool"], "storage");
    assert!(val["rules"].is_array());
    assert_eq!(val["exitCodes"]["0"], "ok");
    assert_eq!(val["exitCodes"]["6"], "invalid_input - bad arguments, fix the call");
}

#[test]
fn test_accounts_add_and_list() {
    let mock = Mock::start(200);
    let env = TestEnv::new(&mock);

    // Add profile
    let out = env.run(&[
        "accounts",
        "add",
        "ivy-tendril",
        "--account-name",
        "stivytelemetry",
        "--container",
        "ivy-tendril",
        "--key",
        "mock-account-key",
    ]);
    assert!(out.status.success());

    // List profiles
    let out_list = env.run(&["accounts", "list", "--json"]);
    assert!(out_list.status.success());
    let list_val: Value = serde_json::from_slice(&out_list.stdout).unwrap();
    assert!(list_val.is_array());
    assert_eq!(list_val.as_array().unwrap().len(), 1);
    assert_eq!(list_val[0]["profile"], "ivy-tendril");
    assert_eq!(list_val[0]["account_name"], "stivytelemetry");
    assert_eq!(list_val[0]["container_name"], "ivy-tendril");
    assert_eq!(list_val[0]["has_stored_key"], true);
}

#[test]
fn test_upload_single_file() {
    let mock = Mock::start(201);
    let env = TestEnv::new(&mock);

    // Configure profile
    let _ = env.run(&["accounts", "add", "testprofile", "--account-name", "ststorage", "--container", "docs"]);

    // Create a sample file
    let sample_file = env.dir.join("report.pdf");
    fs::write(&sample_file, b"%PDF-1.4 sample content").unwrap();

    let out = env.run(&["upload", "testprofile", sample_file.to_str().unwrap(), "--json"]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let val: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(val["status"], "uploaded");
    assert_eq!(val["account"], "ststorage");
    assert_eq!(val["container"], "docs");
    assert_eq!(val["blob"], "report.pdf");
    assert_eq!(val["content_type"], "application/pdf");
    assert!(val["url"].as_str().unwrap().contains("report.pdf"));

    // Check request received by mock server
    let reqs = mock.requests();
    assert_eq!(reqs.len(), 1);
    let r = &reqs[0];
    assert_eq!(r.method, "PUT");
    assert!(r.path.starts_with("/docs/report.pdf"));
    assert_eq!(r.body, b"%PDF-1.4 sample content");

    let blob_type_header = r.headers.iter().find(|(k, _)| k == "x-ms-blob-type");
    assert!(blob_type_header.is_some());
    assert_eq!(blob_type_header.unwrap().1, "BlockBlob");
}

#[test]
fn test_upload_directory_auto_zipped() {
    let mock = Mock::start(201);
    let env = TestEnv::new(&mock);

    let _ = env.run(&["accounts", "add", "ivy-tendril", "--account-name", "stivy", "--container", "bundles"]);

    // Create a folder to upload
    let upload_dir = env.dir.join("bundle-assets");
    let sub = upload_dir.join("sub");
    let excluded = upload_dir.join("node_modules");
    fs::create_dir_all(&sub).unwrap();
    fs::create_dir_all(&excluded).unwrap();
    fs::write(upload_dir.join("main.js"), b"console.log('test');").unwrap();
    fs::write(sub.join("style.css"), b"body { color: red; }").unwrap();
    fs::write(excluded.join("pkg.json"), b"excluded").unwrap();

    let out = env.run(&["upload", "ivy-tendril", upload_dir.to_str().unwrap()]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let reqs = mock.requests();
    assert_eq!(reqs.len(), 1);
    let r = &reqs[0];
    assert_eq!(r.method, "PUT");
    assert!(r.path.starts_with("/bundles/bundle-assets.zip"));

    let content_type = r.headers.iter().find(|(k, _)| k == "content-type").unwrap();
    assert_eq!(content_type.1, "application/zip");
}

#[test]
fn test_upload_nonexistent_file() {
    let mock = Mock::start(200);
    let env = TestEnv::new(&mock);

    let _ = env.run(&["accounts", "add", "myprofile"]);
    let out = env.run(&["upload", "myprofile", "/definitely/nonexistent/file.txt"]);
    assert_eq!(out.status.code(), Some(6)); // InvalidInput
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("invalid_input"));
    assert!(stderr.contains("Path not found"));
}

#[test]
fn test_upload_unconfigured_profile() {
    let mock = Mock::start(200);
    let env = TestEnv::new(&mock);

    let test_file = env.dir.join("file.txt");
    fs::write(&test_file, b"content").unwrap();

    let out = env.run(&["upload", "missingprofile", test_file.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(7)); // NoAccount
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no_account"));
    assert!(stderr.contains("No storage profile named 'missingprofile'"));
}

#[test]
fn test_upload_azure_auth_error_mapping() {
    let mock = Mock::start(401);
    let env = TestEnv::new(&mock);

    let _ = env.run(&["accounts", "add", "authprofile", "--account-name", "stauth", "--container", "box"]);
    let sample = env.dir.join("data.csv");
    fs::write(&sample, b"a,b,c").unwrap();

    let out = env.run(&["upload", "authprofile", sample.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(3)); // AuthRequired
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("auth_required"));
}
