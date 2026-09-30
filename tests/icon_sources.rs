use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn bad_linked_images_fall_through_to_the_site_favicon() {
    let root = std::env::temp_dir().join(format!(
        "tack-icon-sources-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let expected = b"\xef\xbb\xbf  <svg xmlns=\"http://www.w3.org/2000/svg\"><title>Fallback source</title></svg>";
    for (name, bad) in [
        ("Bad HTML", b"<!doctype html>Error page".to_vec()),
        ("Oversized", {
            let mut bytes = vec![0; 6 * 1024 * 1024];
            bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
            bytes
        }),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut requests = 0;
            while requests < 3 && Instant::now() < deadline {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("{error}"),
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = [0; 4096];
                let length = stream.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..length]);
                let body: &[u8] = if request.starts_with("GET /bad.png ") {
                    &bad
                } else if request.starts_with("GET /favicon.ico ") {
                    expected
                } else {
                    b"<html><link rel=\"icon\" href=\"/bad.png\"></html>"
                };
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(body);
                requests += 1;
            }
            requests
        });
        let output = Command::new(env!("CARGO_BIN_EXE_tack"))
            .args([&url, name, "--browser", "/bin/true"])
            .env("XDG_DATA_HOME", &root)
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("HTTPS_PROXY", "http://127.0.0.1:1")
            .env("HTTP_PROXY", "http://127.0.0.1:1")
            .env("ALL_PROXY", "http://127.0.0.1:1")
            .env("NO_PROXY", "127.0.0.1")
            .env("https_proxy", "http://127.0.0.1:1")
            .env("http_proxy", "http://127.0.0.1:1")
            .env("all_proxy", "http://127.0.0.1:1")
            .env("no_proxy", "127.0.0.1")
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(server.join().unwrap(), 3);
        let slug = name.to_lowercase().replace(' ', "-");
        assert_eq!(
            std::fs::read(root.join(format!("icons/{slug}.svg"))).unwrap(),
            expected
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
