// Example server that uses `fixed_buffer` crate to parse a simple line-based protocol.

use std::println;

async fn handle_request(
    tcp_stream: &mut tokio::net::TcpStream,
    request: &str,
) -> Result<(), std::io::Error> {
    match request {
        "VERSION" => {
            let response = "example_server-0.1\n";
            tokio::io::AsyncWriteExt::write_all(tcp_stream, response.as_bytes()).await?;
        }
        "TIME" => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_e| std::io::Error::from(std::io::ErrorKind::Other))?
                .as_secs();
            let response = format!("{}\n", now);
            tokio::io::AsyncWriteExt::write_all(tcp_stream, response.as_bytes()).await?;
        }
        _ => {
            tokio::io::AsyncWriteExt::write_all(tcp_stream, "UNKNOWN COMMAND\n".as_bytes()).await?;
        }
    }
    Ok(())
}

async fn handle_conn(mut tcp_stream: tokio::net::TcpStream) -> Result<(), std::io::Error> {
    println!("SERVER handling connection");
    let mut buf: fixed_buffer::FixedBuf = fixed_buffer::FixedBuf::new();
    loop {
        let line_bytes = buf.read_delimited(&mut tcp_stream, b"\n").await?;
        let line = std::str::from_utf8(line_bytes)
            .map_err(|_e| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
        println!("SERVER got request {:?}", line);
        handle_request(&mut tcp_stream, line).await?;
    }
}

#[tokio::main]
pub async fn main() -> Result<(), std::io::Error> {
    let mut listener =
        tokio::net::TcpListener::bind(std::net::SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();
    let addr = listener.local_addr().unwrap();
    println!("SERVER listening on {}", addr);
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((tcp_stream, _addr)) => {
                    tokio::spawn(handle_conn(tcp_stream));
                }
                Err(e) => {
                    println!("SERVER error accepting connection: {:?}", e);
                    tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    });

    println!("CLIENT connecting");
    let mut tcp_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    println!("CLIENT sending requests");
    tokio::io::AsyncWriteExt::write_all(&mut tcp_stream, b"VERSION\nTIME\n")
        .await
        .unwrap();
    tcp_stream.shutdown(std::net::Shutdown::Write).unwrap();
    let mut response = String::new();
    tokio::io::AsyncReadExt::read_to_string(&mut tcp_stream, &mut response)
        .await
        .unwrap();
    for line in response.lines() {
        println!("CLIENT got response {:?}", line);
    }
    Ok(())
}

// $ cargo run --package fixed-buffer --example server
// SERVER listening on 127.0.0.1:56301
// CLIENT connecting
// CLIENT sending requests
// SERVER handling connection
// SERVER got request "VERSION"
// SERVER got request "TIME"
// CLIENT got response "example_server-0.1"
// CLIENT got response "1602756416"
