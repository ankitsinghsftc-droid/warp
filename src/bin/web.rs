use local_ip_address::list_afinet_netifas;
use rand::Rng;

use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};

const PORT: u16 = 8080;
const BUFFER_SIZE: usize = 64 * 1024;

// ---------------------------------------------------------
// Path helpers
// ---------------------------------------------------------

fn warp_root_folder() -> PathBuf {
    if let Ok(profile) = env::var("USERPROFILE") {
        return PathBuf::from(profile)
            .join("Downloads")
            .join("Warp");
    }

    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home)
            .join("Downloads")
            .join("Warp");
    }

    PathBuf::from("Warp")
}

fn received_folder() -> PathBuf {
    warp_root_folder().join("Received")
}

fn share_folder() -> PathBuf {
    warp_root_folder().join("Share")
}

fn unique_file_path(folder: &Path, filename: &str) -> PathBuf {
    let original = folder.join(filename);

    if !original.exists() {
        return original;
    }

    let path = Path::new(filename);

    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();

    let extension = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_string());

    let mut number = 1;

    loop {
        let new_name = match &extension {
            Some(ext) => {
                format!("{} ({number}).{}", stem, ext)
            }

            None => {
                format!("{} ({number})", stem)
            }
        };

        let candidate = folder.join(new_name);

        if !candidate.exists() {
            return candidate;
        }

        number += 1;
    }
}

// ---------------------------------------------------------
// Filename safety
// ---------------------------------------------------------

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();

    let mut output = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
        {
            if let Ok(hex) =
                std::str::from_utf8(
                    &bytes[index + 1..index + 3],
                )
            {
                if let Ok(value) =
                    u8::from_str_radix(hex, 16)
                {
                    output.push(value);
                    index += 3;
                    continue;
                }
            }
        }

        output.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&output).to_string()
}

fn percent_encode(input: &str) -> String {
    let mut output = String::new();

    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => {
                output.push(*byte as char);
            }

            _ => {
                output.push_str(
                    &format!("%{:02X}", byte)
                );
            }
        }
    }

    output
}

fn query_parameter(
    target: &str,
    wanted: &str,
) -> Option<String> {
    let (_, query) = target.split_once('?')?;

    for pair in query.split('&') {
        let (key, value) =
            pair.split_once('=')
                .unwrap_or((pair, ""));

        if key == wanted {
            return Some(
                percent_decode(value)
            );
        }
    }

    None
}

fn safe_filename(name: &str) -> String {
    let filename = Path::new(name)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    filename
        .chars()
        .filter(|c| {
            !matches!(
                c,
                '<'
                    | '>'
                    | ':'
                    | '"'
                    | '/'
                    | '\\'
                    | '|'
                    | '?'
                    | '*'
            )
        })
        .collect()
}

// ---------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------

fn send_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let headers = format!(
        "HTTP/1.1 {}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         Cache-Control: no-store\r\n\
         X-Content-Type-Options: nosniff\r\n\
         \r\n",
        status,
        content_type,
        body.len()
    );

    stream.write_all(headers.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;

    Ok(())
}

fn read_headers(
    stream: &mut TcpStream,
) -> std::io::Result<Vec<u8>> {
    let mut data = Vec::new();

    let mut byte = [0u8; 1];

    loop {
        stream.read_exact(&mut byte)?;

        data.push(byte[0]);

        if data.len() >= 4
            && &data[data.len() - 4..]
                == b"\r\n\r\n"
        {
            break;
        }

        if data.len() > 64 * 1024 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "HTTP headers too large",
            ));
        }
    }

    Ok(data)
}

fn get_header(
    headers: &str,
    wanted: &str,
) -> Option<String> {
    for line in headers.lines() {
        if let Some((name, value)) =
            line.split_once(':')
        {
            if name
                .trim()
                .eq_ignore_ascii_case(wanted)
            {
                return Some(
                    value.trim().to_string()
                );
            }
        }
    }

    None
}

// ---------------------------------------------------------
// File receiving
// ---------------------------------------------------------

fn receive_file(
    stream: &mut TcpStream,
    headers: &str,
    pair_code: &str,
    download_folder: &Path,
) -> std::io::Result<()> {
    // ---------------- Pair authentication ----------------

    let supplied_code =
        get_header(headers, "X-Warp-Code")
            .unwrap_or_default();

    if supplied_code != pair_code {
        println!("Rejected transfer: wrong pairing code");

        send_response(
            stream,
            "403 Forbidden",
            "text/plain; charset=utf-8",
            b"Wrong pairing code",
        )?;

        return Ok(());
    }

    // ---------------- File size ----------------

    let content_length =
        get_header(headers, "Content-Length")
            .and_then(|value| {
                value.parse::<u64>().ok()
            })
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Missing Content-Length",
                )
            })?;

    // ---------------- Filename ----------------

    let encoded_name =
        get_header(headers, "X-Warp-Filename")
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Missing filename",
                )
            })?;

    let decoded_name =
        percent_decode(&encoded_name);

    let filename =
        safe_filename(&decoded_name);

    if filename.is_empty() {
        send_response(
            stream,
            "400 Bad Request",
            "text/plain",
            b"Invalid filename",
        )?;

        return Ok(());
    }

    fs::create_dir_all(download_folder)?;

    let output_path =
        unique_file_path(
            download_folder,
            &filename,
        );

    println!();
    println!("--------------------------------");
    println!("Incoming transfer");
    println!("File : {}", filename);
    println!("Size : {}", format_bytes(content_length));
    println!("--------------------------------");

    // Write to a temporary partial file first.

    let temp_path = output_path.with_extension(
        match output_path.extension() {
            Some(ext) => {
                format!(
                    "{}.warppart",
                    ext.to_string_lossy()
                )
            }

            None => "warppart".to_string(),
        },
    );

    let mut file = File::create(&temp_path)?;

    let mut buffer =
        [0u8; BUFFER_SIZE];

    let mut remaining =
        content_length;

    let mut received =
        0u64;

    let mut last_percent =
        0u64;

    while remaining > 0 {
        let wanted =
            std::cmp::min(
                remaining,
                BUFFER_SIZE as u64,
            ) as usize;

        let amount =
            stream.read(
                &mut buffer[..wanted]
            )?;

        if amount == 0 {
            drop(file);

            let _ =
                fs::remove_file(&temp_path);

            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Sender disconnected",
                ),
            );
        }

        file.write_all(
            &buffer[..amount]
        )?;

        received += amount as u64;

        remaining -= amount as u64;

        let percent =
            received * 100
                / content_length.max(1);

        if percent >= last_percent + 10
            || percent == 100
        {
            println!(
                "{:>3}%   {} / {}",
                percent,
                format_bytes(received),
                format_bytes(content_length)
            );

            last_percent = percent;
        }
    }

    file.flush()?;

    drop(file);

    // Only expose the real filename once transfer
    // has finished successfully.

    fs::rename(
        &temp_path,
        &output_path,
    )?;

    println!();
    println!(
        "✓ Received: {}",
        output_path.display()
    );
    println!();

    send_response(
        stream,
        "200 OK",
        "text/plain; charset=utf-8",
        b"Transfer complete",
    )?;

    Ok(())
}

// ---------------------------------------------------------
// Human-readable sizes
// ---------------------------------------------------------

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let value = bytes as f64;

    if value >= GB {
        format!("{:.2} GB", value / GB)
    } else if value >= MB {
        format!("{:.1} MB", value / MB)
    } else if value >= KB {
        format!("{:.1} KB", value / KB)
    } else {
        format!("{} B", bytes)
    }
}

// ---------------------------------------------------------
// Sender website
// ---------------------------------------------------------

fn sender_page(
    share_folder: &Path,
) -> String {
    let shared_files =
        shared_files_html(
            share_folder
        );

    let page =
        r#"<!DOCTYPE html>

<html>

<head>

<meta charset="UTF-8">

<meta
    name="viewport"
    content="width=device-width, initial-scale=1.0"
>

<title>Warp</title>

<style>

* {
    box-sizing: border-box;
}

body {
    margin: 0;
    min-height: 100vh;

    display: flex;
    align-items: center;
    justify-content: center;

    padding: 24px;

    font-family:
        -apple-system,
        BlinkMacSystemFont,
        "Segoe UI",
        sans-serif;

    background: #09090b;
    color: #fafafa;
}

.card {
    width: 100%;
    max-width: 520px;

    padding: 32px;

    border:
        1px solid #27272a;

    border-radius: 24px;

    background: #111113;
}

.logo {
    font-size: 38px;
    font-weight: 750;

    letter-spacing: -1.5px;
}

.subtitle {
    margin-top: 6px;
    margin-bottom: 32px;

    color: #a1a1aa;
}

label {
    display: block;

    margin-top: 22px;
    margin-bottom: 8px;

    font-size: 14px;
    color: #a1a1aa;
}

.code {
    width: 100%;

    padding: 16px;

    border:
        1px solid #3f3f46;

    border-radius: 14px;

    outline: none;

    background: #18181b;
    color: white;

    font-size: 24px;

    letter-spacing: 8px;

    text-align: center;
}

.file-box {
    display: block;

    width: 100%;

    padding: 24px;

    border:
        1px dashed #52525b;

    border-radius: 16px;

    background: #18181b;

    text-align: center;
}

input[type=file] {
    width: 100%;
}

button {
    width: 100%;

    margin-top: 24px;

    padding: 17px;

    border: none;

    border-radius: 14px;

    font-size: 17px;
    font-weight: 650;

    cursor: pointer;
}

button:disabled {
    opacity: 0.5;
}

progress {
    width: 100%;
    height: 12px;

    margin-top: 28px;
}

.status {
    min-height: 24px;

    margin-top: 12px;

    color: #a1a1aa;

    text-align: center;
}

.detail {
    margin-top: 8px;

    font-size: 13px;

    color: #71717a;

    text-align: center;
}

.divider {
    height: 1px;
    background: #27272a;

    margin-top: 34px;
    margin-bottom: 26px;
}

.section-title {
    font-size: 17px;
    font-weight: 650;

    margin-bottom: 16px;
}

.shared-file {
    display: flex;

    align-items: center;
    justify-content: space-between;

    gap: 15px;

    padding: 15px;

    margin-top: 10px;

    border: 1px solid #27272a;
    border-radius: 14px;

    background: #18181b;
}

.file-info {
    min-width: 0;
    flex: 1;
}

.file-name {
    overflow: hidden;

    text-overflow: ellipsis;
    white-space: nowrap;

    font-size: 15px;
}

.file-size {
    margin-top: 5px;

    font-size: 12px;

    color: #71717a;
}

.download-button {
    width: auto;

    margin: 0;

    padding: 10px 14px;

    font-size: 14px;
}

.refresh-button {
    margin-top: 16px;

    background: #27272a;

    color: white;
}

.empty {
    padding: 20px;

    border: 1px dashed #3f3f46;
    border-radius: 14px;

    color: #71717a;

    text-align: center;

    font-size: 14px;
}

</style>

</head>

<body>

<div class="card">

    <div class="logo">
        Warp
    </div>

    <div class="subtitle">
        Direct local file transfer.
        No cloud.
    </div>

    <label>
        Pairing code
    </label>

    <input
        id="code"
        class="code"
        maxlength="6"
        inputmode="numeric"
        placeholder="000000"
    >

    <label>
        File
    </label>

    <div class="file-box">

        <input
            id="file"
            type="file"
        >

    </div>

    <button
        id="sendButton"
        onclick="sendFile()"
    >
        Send to this device
    </button>

    <progress
        id="progress"
        value="0"
        max="100"
    >
    </progress>

    <div
        id="status"
        class="status"
    >
        Ready
    </div>

    <div
        id="detail"
        class="detail"
    >
    </div>

    <div class="divider"></div>

    <div class="section-title">
        Files from this device
    </div>

    <div id="sharedFiles">
        {{SHARED_FILES}}
    </div>

    <button
        class="refresh-button"
        onclick="location.reload()"
    >
        Refresh files
    </button>

</div>

<script>

function formatBytes(bytes) {

    if (bytes < 1024)
        return bytes + " B";

    if (bytes < 1024 * 1024)
        return (
            bytes / 1024
        ).toFixed(1) + " KB";

    if (
        bytes <
        1024 * 1024 * 1024
    )
        return (
            bytes /
            1024 /
            1024
        ).toFixed(1) + " MB";

    return (
        bytes /
        1024 /
        1024 /
        1024
    ).toFixed(2) + " GB";
}

function sendFile() {

    const picker =
        document.getElementById(
            "file"
        );

    const code =
        document.getElementById(
            "code"
        ).value.trim();

    const progress =
        document.getElementById(
            "progress"
        );

    const status =
        document.getElementById(
            "status"
        );

    const detail =
        document.getElementById(
            "detail"
        );

    const button =
        document.getElementById(
            "sendButton"
        );

    if (code.length !== 6) {

        status.textContent =
            "Enter the 6-digit pairing code.";

        return;
    }

    if (
        picker.files.length === 0
    ) {

        status.textContent =
            "Choose a file first.";

        return;
    }

    const file =
        picker.files[0];

    const xhr =
        new XMLHttpRequest();

    xhr.open(
        "POST",
        "/upload"
    );

    xhr.setRequestHeader(
        "X-Warp-Code",
        code
    );

    xhr.setRequestHeader(
        "X-Warp-Filename",
        encodeURIComponent(
            file.name
        )
    );

    xhr.setRequestHeader(
        "Content-Type",
        "application/octet-stream"
    );

    button.disabled = true;

    status.textContent =
        "Sending " + file.name;

    detail.textContent =
        formatBytes(file.size);

    xhr.upload.onprogress =
        function(event) {

            if (
                !event.lengthComputable
            )
                return;

            const percent =
                Math.round(
                    event.loaded /
                    event.total *
                    100
                );

            progress.value =
                percent;

            status.textContent =
                percent +
                "% uploaded";

            detail.textContent =
                formatBytes(
                    event.loaded
                ) +
                " / " +
                formatBytes(
                    event.total
                );
        };

    xhr.onload =
        function() {

            button.disabled = false;

            if (
                xhr.status === 200
            ) {

                progress.value = 100;

                status.textContent =
                    "✓ Transfer complete";

                detail.textContent =
                    file.name;

            } else if (
                xhr.status === 403
            ) {

                progress.value = 0;

                status.textContent =
                    "Wrong pairing code";

                detail.textContent = "";

            } else {

                status.textContent =
                    "Transfer failed";

                detail.textContent =
                    "HTTP " +
                    xhr.status;
            }
        };

    xhr.onerror =
        function() {

            button.disabled = false;

            status.textContent =
                "Connection lost";

            detail.textContent =
                "Keep both devices on the same local network.";
        };

    xhr.send(file);
}

function downloadFile(encodedFilename) {

    const code =
        document.getElementById(
            "code"
        ).value.trim();

    const status =
        document.getElementById(
            "status"
        );

    if (code.length !== 6) {

        status.textContent =
            "Enter the pairing code first.";

        return;
    }

    const url =
        "/download?file=" +
        encodedFilename +
        "&code=" +
        encodeURIComponent(code);

    const link =
        document.createElement(
            "a"
        );

    link.href = url;

    document.body.appendChild(
        link
    );

    link.click();

    link.remove();

    status.textContent =
        "Download started";
}

</script>

</body>

</html>"#;

    page.replace(
        "{{SHARED_FILES}}",
        &shared_files,
    )
}

// ---------------------------------------------------------
// Connection handler
// ---------------------------------------------------------

fn shared_files_html(
    share_folder: &Path,
) -> String {

    let mut files =
        Vec::new();

    if let Ok(entries) =
        fs::read_dir(
            share_folder
        )
    {
        for entry in entries.flatten() {
            let path =
                entry.path();

            if !path.is_file() {
                continue;
            }

            let filename =
                entry.file_name()
                    .to_string_lossy()
                    .to_string();

            let size =
                entry.metadata()
                    .map(|m| m.len())
                    .unwrap_or(0);

            files.push(
                (
                    filename,
                    size,
                )
            );
        }
    }

    files.sort_by(
        |a, b|
        a.0
            .to_lowercase()
            .cmp(
                &b.0.to_lowercase()
            )
    );

    if files.is_empty() {
        return r#"
            <div class="empty">
                No files are being shared by this device.
            </div>
        "#
        .to_string();
    }

    let mut html =
        String::new();

    for (filename, size) in files {

        let encoded =
            percent_encode(
                &filename
            );

        let safe_display =
            filename
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;");

        html.push_str(
            &format!(
                r#"
                <div class="shared-file">

                    <div class="file-info">

                        <div class="file-name">
                            {}
                        </div>

                        <div class="file-size">
                            {}
                        </div>

                    </div>

                    <button
                        class="download-button"
                        onclick="downloadFile('{}')"
                    >
                        Download
                    </button>

                </div>
                "#,
                safe_display,
                format_bytes(size),
                encoded
            )
        );
    }

    html
}

fn send_shared_file(
    stream: &mut TcpStream,
    request_target: &str,
    pair_code: &str,
    share_folder: &Path,
) -> std::io::Result<()> {

    // ---------------- Pair code ----------------

    let supplied_code =
        query_parameter(
            request_target,
            "code",
        )
        .unwrap_or_default();

    if supplied_code != pair_code {
        return send_response(
            stream,
            "403 Forbidden",
            "text/plain; charset=utf-8",
            b"Wrong pairing code",
        );
    }

    // ---------------- Filename ----------------

    let requested_name =
        query_parameter(
            request_target,
            "file",
        )
        .unwrap_or_default();

    let filename =
        safe_filename(
            &requested_name
        );

    if filename.is_empty() {
        return send_response(
            stream,
            "400 Bad Request",
            "text/plain",
            b"Invalid filename",
        );
    }

    let path =
        share_folder.join(
            &filename
        );

    if !path.exists()
        || !path.is_file()
    {
        return send_response(
            stream,
            "404 Not Found",
            "text/plain",
            b"File not found",
        );
    }

    let mut file =
        File::open(&path)?;

    let file_size =
        file.metadata()?.len();

    println!();
    println!("--------------------------------");
    println!("Outgoing transfer");
    println!("File : {}", filename);
    println!("Size : {}", format_bytes(file_size));
    println!("--------------------------------");

    // Encoded filename is safe to place
    // inside the HTTP header.
    let encoded_name =
        percent_encode(
            &filename
        );

    let headers = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/octet-stream\r\n\
         Content-Length: {}\r\n\
         Content-Disposition: attachment; filename=\"warp-download\"; filename*=UTF-8''{}\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\
         X-Content-Type-Options: nosniff\r\n\
         \r\n",
        file_size,
        encoded_name
    );

    stream.write_all(
        headers.as_bytes()
    )?;

    // ---------------- Streaming ----------------

    let mut buffer =
        [0u8; BUFFER_SIZE];

    let mut total_sent =
        0u64;

    let mut last_percent =
        0u64;

    loop {
        let amount =
            file.read(
                &mut buffer
            )?;

        if amount == 0 {
            break;
        }

        stream.write_all(
            &buffer[..amount]
        )?;

        total_sent +=
            amount as u64;

        let percent =
            total_sent * 100
                / file_size.max(1);

        if percent >= last_percent + 10
            || percent == 100
        {
            println!(
                "{:>3}%   {} / {}",
                percent,
                format_bytes(total_sent),
                format_bytes(file_size)
            );

            last_percent =
                percent;
        }
    }

    stream.flush()?;

    println!();
    println!(
        "✓ Sent: {}",
        filename
    );
    println!();

    Ok(())
}

fn handle_connection(
    mut stream: TcpStream,
    pair_code: &str,
    received_folder: &Path,
    share_folder: &Path,
) -> std::io::Result<()> {
    let header_bytes =
        read_headers(&mut stream)?;

    let headers =
        String::from_utf8_lossy(
            &header_bytes
        );

    let first_line =
        headers.lines()
            .next()
            .unwrap_or("");

    let request_target =
        first_line
            .split_whitespace()
            .nth(1)
            .unwrap_or("/");

    if request_target.starts_with(
        "/download?"
    ) {
        return send_shared_file(
            &mut stream,
            request_target,
            pair_code,
            share_folder,
        );
    }

    if first_line.starts_with(
        "POST /upload "
    ) {
        return receive_file(
            &mut stream,
            &headers,
            pair_code,
            received_folder,
        );
    }

    if first_line.starts_with(
        "GET / "
    ) {
        let page =
            sender_page(
                share_folder
            );

        return send_response(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            page.as_bytes(),
        );
    }

    // Browsers often request this automatically.
    if first_line.starts_with(
        "GET /favicon.ico "
    ) {
        return send_response(
            &mut stream,
            "204 No Content",
            "text/plain",
            b"",
        );
    }

    send_response(
        &mut stream,
        "404 Not Found",
        "text/plain",
        b"Not found",
    )
}

// ---------------------------------------------------------
// Network interfaces
// ---------------------------------------------------------

fn available_urls() -> Vec<String> {
    let mut urls =
        Vec::new();

    if let Ok(interfaces) =
        list_afinet_netifas()
    {
        for (_name, ip) in interfaces {
            if let IpAddr::V4(ipv4) = ip {
                if ipv4.is_loopback() {
                    continue;
                }

                // Ignore Windows automatic
                // fallback addresses.
                if ipv4.octets()[0] == 169
                    && ipv4.octets()[1] == 254
                {
                    continue;
                }

                urls.push(
                    format!(
                        "http://{}:{}",
                        ipv4,
                        PORT
                    )
                );
            }
        }
    }

    urls.sort();
    urls.dedup();

    urls
}

// ---------------------------------------------------------
// Main
// ---------------------------------------------------------

fn main() {
    let pair_code =
        format!(
            "{:06}",
            rand::thread_rng()
                .gen_range(
                    0..1_000_000
                )
        );

    let root_folder =
        warp_root_folder();

    let received_folder =
        received_folder();

    let share_folder =
        share_folder();

    fs::create_dir_all(
        &received_folder
    )
    .expect(
        "Could not create Received folder"
    );

    fs::create_dir_all(
        &share_folder
    )
    .expect(
        "Could not create Share folder"
    );

    let listener =
        TcpListener::bind(
            ("0.0.0.0", PORT)
        )
        .expect(
            "Could not start Warp.\n\
             Port 8080 may already be in use."
        );

    println!();
    println!(
        "======================================"
    );
    println!(
        "                 WARP"
    );
    println!(
        "======================================"
    );
    println!();

    println!(
        "Pairing code: {}",
        pair_code
    );

    println!();

    println!(
        "Open one of these addresses on the sender:"
    );

    let urls =
        available_urls();

    if urls.is_empty() {
        println!(
            "Could not detect a local IP address."
        );
    } else {
        for url in urls {
            println!(
                "  {}",
                url
            );
        }
    }

    println!();

    println!(
        "Files received from other devices:"
    );

    println!(
        "  {}",
        received_folder.display()
    );

    println!();

    println!(
        "Files available for others to download:"
    );

    println!(
        "  {}",
        share_folder.display()
    );

    println!();

    println!(
        "Keep this window open while receiving."
    );

    println!(
        "Press Ctrl+C to stop Warp."
    );

    println!();

    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                if let Err(error) =
                    handle_connection(
                        stream,
                        &pair_code,
                        &received_folder,
                        &share_folder,
                    )
                {
                    println!(
                        "Transfer error: {}",
                        error
                    );
                }
            }

            Err(error) => {
                println!(
                    "Connection error: {}",
                    error
                );
            }
        }
    }
}