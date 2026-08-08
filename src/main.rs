use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

fn handle_connection(
    mut stream: TcpStream,
) -> std::io::Result<()> {

    println!("Device connected!");

    // --------------------------------
    // 1. Receive filename length
    // --------------------------------

    let mut name_length_buffer = [0u8; 4];

    stream.read_exact(
        &mut name_length_buffer
    )?;

    let name_length =
        u32::from_be_bytes(
            name_length_buffer
        ) as usize;

    if name_length > 1024 {
        println!("Invalid filename length");
        return Ok(());
    }

    // --------------------------------
    // 2. Receive filename
    // --------------------------------

    let mut name_buffer =
        vec![0u8; name_length];

    stream.read_exact(
        &mut name_buffer
    )?;

    let received_name =
        String::from_utf8_lossy(
            &name_buffer
        );

    // Prevent paths like ../../secret.txt

    let safe_name =
        Path::new(received_name.as_ref())
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

    println!("Incoming file: {}", safe_name);

    // --------------------------------
    // 3. Receive file size
    // --------------------------------

    let mut size_buffer = [0u8; 8];

    stream.read_exact(
        &mut size_buffer
    )?;

    let file_size =
        u64::from_be_bytes(
            size_buffer
        );

    println!(
        "File size: {} bytes",
        file_size
    );

    // --------------------------------
    // 4. Create output file
    // --------------------------------

    let output_name =
        format!(
            "received_{}",
            safe_name
        );

    let mut output_file =
        File::create(&output_name)?;

    // --------------------------------
    // 5. Receive file in chunks
    // --------------------------------

    let mut buffer = [0u8; 64 * 1024];

    let mut remaining =
        file_size;

    let mut total_received =
        0u64;

    while remaining > 0 {

        let amount_to_read =
            std::cmp::min(
                buffer.len() as u64,
                remaining
            ) as usize;

        stream.read_exact(
            &mut buffer[..amount_to_read]
        )?;

        output_file.write_all(
            &buffer[..amount_to_read]
        )?;

        remaining -=
            amount_to_read as u64;

        total_received +=
            amount_to_read as u64;

        println!(
            "Received {} / {} bytes",
            total_received,
            file_size
        );
    }

    println!(
        "Saved as: {}",
        output_name
    );

    Ok(())
}

fn main() {
    let listener =
        TcpListener::bind(
            "0.0.0.0:7878"
        )
        .expect(
            "Failed to start Warp receiver"
        );

    println!(
        "Warp receiver listening on port 7878..."
    );

    for connection in listener.incoming() {

        match connection {

            Ok(stream) => {

                if let Err(error) =
                    handle_connection(stream)
                {
                    println!(
                        "Transfer failed: {}",
                        error
                    );
                }
            }

            Err(error) => {
                println!(
                    "Connection failed: {}",
                    error
                );
            }
        }
    }
}