use std::{
    collections::HashMap, error::Error, fs, io::prelude::*, net::TcpListener, net::TcpStream,
    thread, time::Duration, time::Instant,
};

use hello::ThreadPool;

const CRLF: &str = "\r\n";

fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:7878")?;
    let pool = ThreadPool::new(4);
    println!("👂 listening on port 7878");

    for stream in listener.incoming() {
        let stream: TcpStream = stream?;

        let now = Instant::now();
        pool.execute(move || {
            handle_connection(now, stream);
        });
    }

    println!("bye bye 👋");
    Ok(())
}

fn handle_connection(now: Instant, mut stream: TcpStream) {
    println!(
        "⏳ starting request processing, elapsed: {:?}",
        now.elapsed()
    );
    let mut buffer = [0; 1024];

    stream.read(&mut buffer).unwrap();

    let req = String::from_utf8_lossy(&buffer);
    println!("{}", req);

    // HTTP requests have the following format
    //  Method Request-URI HTTP-Version CRLF
    //  headers CRLF
    //  message-body
    let req: Vec<&str> = req
        .lines()
        .next()
        .map(|first_line| first_line.split(' ').collect())
        .unwrap();
    let uri = req[1];

    let response = match uri {
        "/" => http_respond(200, "static/hello.html"),
        "/sleep" => {
            thread::sleep(Duration::from_secs(5));
            http_respond(200, "static/hello.html")
        }
        _ => http_respond(404, "static/404.html"),
    };

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();

    println!(
        "✅ Finished processing request, elapsed {:?}",
        now.elapsed()
    );
}

/// Sends back a http response with the body as the specified html file
/// HTTP responses have the following format:
///     HTTP-Version Status-Code Reason-Phrase CRLF
///     headers CRLF
///     message-body
fn http_respond(status_code: u32, html_file: &str) -> String {
    let mut reasons: HashMap<u32, &str> = HashMap::new();
    reasons.insert(200, "OK");
    reasons.insert(404, "NOT FOUND");

    let reason = reasons.get(&status_code).unwrap_or(&"OK");

    let html = fs::read_to_string(html_file).unwrap();

    format!(
        "\
    HTTP/1.1 {} {}{}\
    Content-Length: {}{}\
    Content-Type: text/html{}{}\
    {}
    ",
        status_code,
        reason,
        CRLF,
        html.len(),
        CRLF,
        CRLF,
        CRLF,
        html
    )
}
