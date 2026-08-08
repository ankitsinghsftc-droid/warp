use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage:");
        println!("cargo run --bin sender -- <file_path>");
        return;
    }

    let path = Path::new(&args[1]);

    let mut file =
        File::open(path)
            .expect("Could not open file");

    let file_size =
        file.metadata()
            .expect("Could not read file metadata")
            .len();

    let file_name =
        path.file_name()
            .expect("Invalid file name")
            .to_string_lossy();

    println!("File: {}", file_name);
    println!("Size: {} bytes", file_size);

    let mut stream =
        TcpStream::connect("127.0.0.1:7878")
            .expect("Could not connect to receiver");

    println!("Connected to Warp receiver");

    // --------------------------------
    // 1. Send filename length
    // --------------------------------

    let file_name_bytes = file_name.as_bytes();

    let file_name_length =
        file_name_bytes.len() as u32;

    stream
        .write_all(&file_name_length.to_be_bytes())
        .expect("Failed to send filename length");

    // --------------------------------
    // 2. Send filename
    // --------------------------------

    stream
        .write_all(file_name_bytes)
        .expect("Failed to send filename");

    // --------------------------------
    // 3. Send file size
    // --------------------------------

    stream
        .write_all(&file_size.to_be_bytes())
        .expect("Failed to send file size");

    // --------------------------------
    // 4. Stream actual file
    // --------------------------------

    let mut buffer = [0u8; 64 * 1024];

    let mut total_sent = 0u64;

    loop {
        let bytes_read =
            file.read(&mut buffer)
                .expect("Failed reading file");

        if bytes_read == 0 {
            break;
        }

        stream
            .write_all(&buffer[..bytes_read])
            .expect("Failed sending file");

        total_sent += bytes_read as u64;

        println!(
            "Sent {} / {} bytes",
            total_sent,
            file_size
        );
    }

    println!("Transfer complete!");
}