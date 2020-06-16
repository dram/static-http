use std::io::prelude::{BufRead, Read, Write};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn content_type(path: &std::path::Path) -> &str {
    match path.extension().map(|s| s.to_str()).flatten() {
        Some("js") => "application/javascript",
        Some("json") => "application/json",

        Some("woff") => "font/woff",

        Some("ico") => "image/x-icon",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",

        Some("css") => "text/css",
        Some("html") => "text/html",

        _ => panic!("Failed to get content type: {}", path.display()),
    }
}

fn main() -> Result<()> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();

    let root = arguments
        .get(0)
        .expect("Usage: static-http document-root [host [port]]");

    let host = arguments.get(1).map_or("127.0.0.1", String::as_str);
    let port = arguments.get(2).map_or("8000", String::as_str);

    serve(root, host, port)
}

fn serve(root: &str, host: &str, port: &str) -> Result<()> {
    eprintln!("Listening on {}:{} ...", host, port);

    let root = std::path::Path::new(root);

    let listener = std::net::TcpListener::bind(format!("{}:{}", host, port))?;

    for stream in listener.incoming() {
        let mut stream = stream?;

        let mut reader = std::io::BufReader::new(&mut stream);

        let mut buffer = String::new();

        reader.read_line(&mut buffer)?;

        eprintln!("Processing {} ...", buffer.trim_end());

        let mut pieces = buffer.split_whitespace();

        if matches!(pieces.next(), Some("GET")) {
            if let Some(uri) = pieces.next() {
                if let Ok(path) = root.join(&uri[1..]).canonicalize() {
                    if path.starts_with(root) {
                        if path.is_dir() {
                            serve_file(&mut stream, &path.join("index.html"))?
                        } else {
                            serve_file(&mut stream, &path)?
                        }
                    } else {
                        serve_status(&mut stream, 403, "Forbidden")?
                    }
                } else {
                    serve_status(&mut stream, 404, "Not Found")?
                }
            } else {
                serve_status(&mut stream, 400, "Bad Request")?
            }
        } else {
            serve_status(&mut stream, 400, "Bad Request")?
        }
    }

    Ok(())
}

fn serve_file(
    stream: &mut std::net::TcpStream,
    path: &std::path::Path,
) -> Result<()> {
    let cache_age: u64 = 3600;

    write!(
        stream,
        "HTTP/1.1 200 OK\r\n\
         Cache-Control: max-age={}\r\n\
         Connection: close\r\n\
         Content-Type: {}\r\n\
         Transfer-Encoding: chunked\r\n\
         \r\n",
        cache_age,
        content_type(path)
    )?;

    let mut data = std::fs::File::open(path)?;

    const BLOCK_SIZE: usize = 4096;

    let mut buffer = [0; BLOCK_SIZE];

    loop {
        let count = data.read(&mut buffer)?;

        if count == 0 {
            break;
        }

        write!(stream, "{:x}\r\n", count)?;
        stream.write_all(&buffer[..count])?;
        stream.write_all(b"\r\n")?;
    }

    Ok(stream.write_all(b"0\r\n\r\n")?)
}

fn serve_status(
    stream: &mut std::net::TcpStream,
    code: i32,
    message: &str,
) -> Result<()> {
    Ok(write!(
        stream,
        "HTTP/1.1 {} {}\r\nConnection: close\r\n\r\n",
        code, message
    )?)
}
