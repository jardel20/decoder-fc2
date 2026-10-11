//! Offline download page and a loopback-only static server.
use crate::error::{Error, Result};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    path::Path,
    time::Duration,
};

#[derive(Debug, Serialize)]
pub struct DownloadWork {
    pub number: u16,
    pub directory: String,
    pub name: String,
    pub records: usize,
    pub groups: usize,
    pub warnings: usize,
    pub filename: Option<String>,
    pub error: Option<String>,
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn safe_filename(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('.')
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

pub fn page(source_name: &str, source_hash: &str, works: &[DownloadWork]) -> Result<Vec<u8>> {
    let mut rows = String::new();
    for work in works {
        let action = if let Some(filename) = &work.filename {
            if !safe_filename(filename) {
                return Err(Error::Format("unsafe download filename".into()));
            }
            format!(
                "<a class=\"button\" href=\"./{filename}\" download aria-label=\"Baixar {} em XLSX\">Baixar XLSX</a>",
                escape(&work.name)
            )
        } else {
            "<span class=\"failed\">Exportação bloqueada</span>".into()
        };
        let status = if let Some(error) = &work.error {
            format!(
                "<span class=\"failed\" title=\"{}\">Erro na validação</span>",
                escape(error)
            )
        } else if work.warnings > 0 {
            "<span class=\"status warn\">Campos indisponíveis</span>".into()
        } else {
            "<span class=\"status\">Dados presentes</span>".into()
        };
        rows.push_str(&format!("<tr><td><span class=\"work-name\">{}</span><span class=\"directory\">{} · nº {}</span></td><td class=\"num\">{}</td><td class=\"num group-cell\">{}</td><td>{status}</td><td>{action}</td></tr>",escape(&work.name),escape(&work.directory),work.number,work.records,work.groups));
    }
    let values = BTreeMap::from([
        ("SOURCE_NAME", escape(source_name)),
        ("WORK_COUNT", works.len().to_string()),
        (
            "RECORD_COUNT",
            works.iter().map(|w| w.records).sum::<usize>().to_string(),
        ),
        (
            "GROUP_COUNT",
            works.iter().map(|w| w.groups).sum::<usize>().to_string(),
        ),
        ("SOURCE_SHA256", escape(source_hash)),
        ("TOOL_VERSION", env!("CARGO_PKG_VERSION").into()),
        ("ROWS", rows),
    ]);
    let mut remaining = include_str!("../assets/downloads.html");
    let mut result = String::new();
    while let Some((prefix, tail)) = remaining.split_once("{{") {
        let (key, tail) = tail
            .split_once("}}")
            .ok_or_else(|| Error::Format("incomplete page template token".into()))?;
        let value = values
            .get(key)
            .ok_or_else(|| Error::Format("unknown page template token".into()))?;
        result.push_str(prefix);
        result.push_str(value);
        remaining = tail;
    }
    result.push_str(remaining);
    Ok(result.into_bytes())
}

fn response(
    stream: &mut impl Write,
    status: &str,
    mime: &str,
    body: &[u8],
    download: Option<&str>,
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; img-src data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'\r\n",
        body.len()
    )?;
    if let Some(name) = download {
        write!(
            stream,
            "Content-Disposition: attachment; filename=\"{name}\"\r\n"
        )?;
    }
    stream.write_all(b"\r\n")?;
    stream.write_all(body)
}

fn handle(
    mut stream: TcpStream,
    files: &BTreeMap<String, Vec<u8>>,
    port: u16,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut bytes = Vec::new();
    let mut part = [0u8; 1024];
    loop {
        let n = stream.read(&mut part)?;
        if n == 0 {
            return Ok(());
        }
        bytes.extend_from_slice(&part[..n]);
        if bytes.len() > 8192 {
            return response(
                &mut stream,
                "431 Request Header Fields Too Large",
                "text/plain; charset=utf-8",
                b"Header too large",
                None,
            );
        }
        if bytes.windows(4).any(|s| s == b"\r\n\r\n") {
            break;
        }
    }
    dispatch(&mut stream, &bytes, files, port)
}

fn host_status(value: &str, port: u16) -> Option<&'static str> {
    let (host, explicit_port) = match value.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (value, None),
    };
    if host.is_empty()
        || host
            .bytes()
            .any(|b| !b.is_ascii_alphanumeric() && b != b'.' && b != b'-')
    {
        return Some("400 Bad Request");
    }
    if let Some(explicit) = explicit_port {
        if explicit.is_empty() || !explicit.bytes().all(|b| b.is_ascii_digit()) {
            return Some("400 Bad Request");
        }
        match explicit.parse::<u16>() {
            Ok(actual) if actual == port => {}
            Ok(_) => return Some("403 Forbidden"),
            Err(_) => return Some("400 Bad Request"),
        }
    }
    if !host.eq_ignore_ascii_case("localhost") && host != "127.0.0.1" {
        return Some("403 Forbidden");
    }
    None
}

fn dispatch(
    stream: &mut impl Write,
    request: &[u8],
    files: &BTreeMap<String, Vec<u8>>,
    port: u16,
) -> std::io::Result<()> {
    let headers = std::str::from_utf8(request)
        .ok()
        .and_then(|text| text.split_once("\r\n\r\n").map(|(headers, _)| headers))
        .unwrap_or("");
    let mut lines = headers.split("\r\n");
    let first = lines.next().unwrap_or("");
    let parts: Vec<_> = first.split_whitespace().collect();
    if parts.len() != 3 || !matches!(parts[2], "HTTP/1.0" | "HTTP/1.1") {
        return response(
            stream,
            "400 Bad Request",
            "text/plain; charset=utf-8",
            b"Malformed request",
            None,
        );
    }
    let mut host = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return response(
                stream,
                "400 Bad Request",
                "text/plain; charset=utf-8",
                b"Malformed header",
                None,
            );
        };
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
            || value.bytes().any(|b| b.is_ascii_control() && b != b'\t')
        {
            return response(
                stream,
                "400 Bad Request",
                "text/plain; charset=utf-8",
                b"Malformed header",
                None,
            );
        }
        if name.eq_ignore_ascii_case("host") {
            if host.is_some() {
                return response(
                    stream,
                    "400 Bad Request",
                    "text/plain; charset=utf-8",
                    b"Duplicate Host",
                    None,
                );
            }
            host = Some(value.trim_matches([' ', '\t']));
        }
    }
    if let Some(host) = host {
        if let Some(status) = host_status(host, port) {
            return response(
                stream,
                status,
                "text/plain; charset=utf-8",
                b"Host is not allowed",
                None,
            );
        }
    } else if parts[2] == "HTTP/1.1" {
        return response(
            stream,
            "400 Bad Request",
            "text/plain; charset=utf-8",
            b"Host is required",
            None,
        );
    }
    if parts[0] != "GET" {
        return response(
            stream,
            "405 Method Not Allowed",
            "text/plain; charset=utf-8",
            b"Only GET is supported",
            None,
        );
    }
    let name = if parts[1] == "/" {
        "index.html"
    } else {
        parts[1].strip_prefix('/').unwrap_or("")
    };
    let Some(body) = files.get(name) else {
        return response(
            stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"File not found",
            None,
        );
    };
    let mime = if name.ends_with(".xlsx") {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    } else if name.ends_with(".zip") {
        "application/zip"
    } else if name.ends_with(".json") {
        "application/json; charset=utf-8"
    } else {
        "text/html; charset=utf-8"
    };
    response(
        stream,
        "200 OK",
        mime,
        body,
        (name != "index.html").then_some(name),
    )
}

/// Serve only in-memory copies of the generated flat download directory.
pub fn serve(directory: &Path, port: u16) -> Result<()> {
    let mut files = BTreeMap::new();
    let mut total = 0usize;
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !safe_filename(&name)
            || !(name == "index.html"
                || name.ends_with(".xlsx")
                || name.ends_with(".json")
                || name.ends_with(".zip"))
        {
            continue;
        }
        let data = crate::reader::read_file(&entry.path())?;
        total = total
            .checked_add(data.len())
            .ok_or_else(|| Error::Format("download directory size overflow".into()))?;
        if total > 256 * 1024 * 1024 {
            return Err(Error::Format("download directory exceeds 256 MiB".into()));
        }
        files.insert(name, data);
    }
    if !files.contains_key("index.html") {
        return Err(Error::Format(
            "download directory must contain the generated index.html".into(),
        ));
    }
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
    let address = listener.local_addr()?;
    println!("FC2-RS: http://{address} — Ctrl+C to stop");
    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                if let Err(error) = handle(stream, &files, address.port()) {
                    eprintln!("HTTP request failed: {error}");
                }
            }
            Err(error) => eprintln!("HTTP connection failed: {error}"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requests_serve_exact_workbook_bytes_and_download_headers() {
        let files = BTreeMap::from([
            ("index.html".into(), b"<html>local</html>".to_vec()),
            ("135-work.xlsx".into(), vec![0x50, 0x4b, 0, 255]),
        ]);
        let mut out = Vec::new();
        dispatch(
            &mut out,
            b"GET /135-work.xlsx HTTP/1.1\r\nHost: localhost\r\n\r\n",
            &files,
            8123,
        )
        .unwrap();
        let split = out.windows(4).position(|b| b == b"\r\n\r\n").unwrap();
        let header = std::str::from_utf8(&out[..split]).unwrap();
        assert!(header.starts_with("HTTP/1.1 200 OK"));
        assert!(header.contains("Content-Disposition: attachment; filename=\"135-work.xlsx\""));
        assert!(header.contains("Content-Length: 4"));
        assert_eq!(&out[split + 4..], &[0x50, 0x4b, 0, 255]);
        out.clear();
        dispatch(
            &mut out,
            b"GET / HTTP/1.1\r\nHost: 127.0.0.1:8123\r\n\r\n",
            &files,
            8123,
        )
        .unwrap();
        assert!(out.ends_with(b"<html>local</html>"));
    }
    #[test]
    fn malformed_requests_uploads_and_path_traversal_do_not_access_files() {
        let files = BTreeMap::from([("index.html".into(), b"safe".to_vec())]);
        for (request, status) in [
            (
                &b"POST /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n"[..],
                "405",
            ),
            (
                &b"GET /../source.FC2 HTTP/1.1\r\nHost: localhost\r\n\r\n"[..],
                "404",
            ),
            (
                &b"GET /%2e%2e/secret HTTP/1.1\r\nHost: localhost\r\n\r\n"[..],
                "404",
            ),
            (&b"broken\r\n\r\n"[..], "400"),
            (&b"\xff\r\n\r\n"[..], "400"),
        ] {
            let mut out = Vec::new();
            dispatch(&mut out, request, &files, 8123).unwrap();
            assert!(
                std::str::from_utf8(&out)
                    .unwrap()
                    .starts_with(&format!("HTTP/1.1 {status}"))
            );
            assert!(!out.ends_with(b"safe"));
        }
    }
    #[test]
    fn host_validation_rejects_rebinding_duplicates_malformed_names_and_wrong_ports() {
        let files = BTreeMap::from([("index.html".into(), b"private data".to_vec())]);
        for (headers, status) in [
            ("", "400"),
            ("Host: evil.example\r\n", "403"),
            ("Host: localhost.evil.example:8123\r\n", "403"),
            ("Host: 127.0.0.1:8124\r\n", "403"),
            ("Host: localhost\r\nHOST: localhost\r\n", "400"),
            ("Host: \r\n", "400"),
            ("Host : localhost\r\n", "400"),
            ("Host: localhost:bad\r\n", "400"),
            ("Host: localhost:\r\n", "400"),
            ("Host: localhost:65536\r\n", "400"),
            ("Host: localhost:8123:8123\r\n", "400"),
            ("Host: localhost@evil.example\r\n", "400"),
            ("Host: localhost,evil.example\r\n", "400"),
            ("Host: localhost\r\n\tcontinued\r\n", "400"),
            ("Host: localhost\0\r\n", "400"),
        ] {
            let request = format!("GET / HTTP/1.1\r\n{headers}\r\n");
            let mut out = Vec::new();
            dispatch(&mut out, request.as_bytes(), &files, 8123).unwrap();
            assert!(
                String::from_utf8_lossy(&out).starts_with(&format!("HTTP/1.1 {status}")),
                "{headers:?}"
            );
            assert!(!out.ends_with(b"private data"));
        }
        for headers in [
            "Host: localhost\r\n",
            "host: LOCALHOST:8123\r\n",
            "Host: 127.0.0.1\r\n",
            "Host: 127.0.0.1:8123\r\n",
            "Host: \tlocalhost:8123 \t\r\n",
        ] {
            let request = format!("GET / HTTP/1.1\r\n{headers}\r\n");
            let mut out = Vec::new();
            dispatch(&mut out, request.as_bytes(), &files, 8123).unwrap();
            assert!(out.starts_with(b"HTTP/1.1 200 OK"));
            assert!(out.ends_with(b"private data"));
        }
    }

    #[test]
    fn http10_without_host_is_supported_and_header_body_cannot_supply_host() {
        let files = BTreeMap::from([("index.html".into(), b"private data".to_vec())]);
        let mut out = Vec::new();
        dispatch(&mut out, b"GET / HTTP/1.0\r\n\r\n", &files, 8123).unwrap();
        assert!(out.starts_with(b"HTTP/1.1 200 OK"));
        assert!(out.ends_with(b"private data"));
        out.clear();
        dispatch(
            &mut out,
            b"GET / HTTP/1.1\r\n\r\nHost: localhost\r\n",
            &files,
            8123,
        )
        .unwrap();
        assert!(out.starts_with(b"HTTP/1.1 400 Bad Request"));
        assert!(!out.ends_with(b"private data"));
        out.clear();
        dispatch(
            &mut out,
            b"GET / HTTP/1.0\r\nHost: evil.example\r\n\r\n",
            &files,
            8123,
        )
        .unwrap();
        assert!(out.starts_with(b"HTTP/1.1 403 Forbidden"));
        assert!(!out.ends_with(b"private data"));
    }
    #[test]
    fn page_escapes_names_without_recursively_expanding_user_tokens() {
        let mut work = DownloadWork {
            number: 1,
            directory: "/trabalho1".into(),
            name: "<script> & trabalho".into(),
            records: 1,
            groups: 1,
            warnings: 0,
            filename: Some("1-work.xlsx".into()),
            error: None,
        };
        let page =
            String::from_utf8(page("{{ROWS}}<script>", &"a".repeat(64), &[work]).unwrap()).unwrap();
        assert!(page.contains("{{ROWS}}&lt;script&gt;"));
        assert!(page.contains("&lt;script&gt; &amp; trabalho"));
        assert_eq!(page.matches("<tr><td>").count(), 1);
        work = DownloadWork {
            number: 1,
            directory: "/trabalho1".into(),
            name: "unsafe".into(),
            records: 1,
            groups: 1,
            warnings: 0,
            filename: Some("../unsafe.xlsx".into()),
            error: None,
        };
        assert!(super::page("source", "hash", &[work]).is_err());
    }
}
