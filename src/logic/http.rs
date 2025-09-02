use std::{
    env,
    fs::OpenOptions,
    io::{BufRead, Read, Write},
    net::TcpStream,
    sync::Arc,
};

use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned, pki_types::ServerName};
use webpki_roots::TLS_SERVER_ROOTS;

pub fn send_web_request() {
    let hostname = env::var("NIGHTCRAB_SOURCE").expect("Host name for data source was not set");
    let http_request = build_request(&hostname);

    let mut root_store = RootCertStore::empty();
    root_store.extend(TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let config = Arc::new(config);
    let server_name = ServerName::try_from(hostname.clone()).expect("failed server name thing");
    let sock = TcpStream::connect(format!("{hostname}:443")).expect("Socket connection failed");
    let conn: ClientConnection =
        ClientConnection::new(config, server_name).expect("TLS connection failed");
    let mut tls = StreamOwned::new(conn, sock);

    tls.write_all(http_request.trim_end().as_bytes())
        .expect("https connection failed");

    tls.conn
        .negotiated_cipher_suite()
        .expect("cipher suite failed");

    let mut string_buf = String::new();
    let mut json_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("weapons.json")
        .expect("file creation failed");

    let mut content_start = false;
    loop {
        let content_line = match tls.read_line(&mut string_buf) {
            Ok(0) => break,
            Ok(buf_size) => buf_size > 6,
            Err(err) => {
                println!("{err}");
                break;
            }
        };

        if string_buf.starts_with('{') {
            content_start = true;
        }

        string_buf.truncate(string_buf.len() - 2);
        if content_start && content_line {
            json_file.write_all(string_buf.as_bytes()).expect("error");
        } else {
            println!("{string_buf}");
        }
        string_buf.clear();
    }
}

fn build_request(hostname: &str) -> String {
    let hostname_formatted = format!("Host: {hostname}");
    let mut base_request = [
        "POST /api/elden-ring-nightreign/v1/graphql/query HTTP/1.1",
        hostname_formatted.as_str(),
        "User-Agent: Wget/1.25.0",
        "Accept: */*",
        "Accept-Encoding: identity",
        "Connection: close",
        "Content-Type: application/json",
        "",
        "",
        "",
    ];

    let mut graphql_part = String::new();
    OpenOptions::new()
        .append(true)
        .create(true)
        .open("./res/Weapons.graphql")
        .expect("file creation failed")
        .read_to_string(&mut graphql_part)
        .expect("failed to write anything into file");
    graphql_part.retain(|c| !c.is_control());
    let json_part = format!(
        "{{\"variables\":{{\"input\":{{\"staticDataTypes\":[\"weapons\"]}}}},\"query\":\"{graphql_part}\"}}"
    );
    let con_len = format!("Content-Length: {}", json_part.len());
    base_request[7] = &con_len;
    base_request[9] = &json_part;
    base_request.join("\r\n")
}
