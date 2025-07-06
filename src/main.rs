use std::{
    fs::{File, read_to_string},
    io::{Read, Write},
    net::TcpStream,
    sync::Arc,
};

use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned, pki_types::ServerName};
use webpki_roots::TLS_SERVER_ROOTS;

fn main() {
    //print!("{}", build_request().trim_end());
    send_web_request();
}

fn send_web_request() {
    let http_request = build_request();
    let hostname = "mobalytics.gg";

    let mut root_store = RootCertStore::empty();
    root_store.extend(TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let config = Arc::new(config);
    let server_name = ServerName::try_from(hostname).expect("failed server naem thing");
    let sock = TcpStream::connect(format!("{}:443", hostname)).expect("Socket connection failed");
    let conn: ClientConnection =
        ClientConnection::new(config, server_name).expect("TLS connection failed");
    let mut tls = StreamOwned::new(conn, sock);

    tls.write_all(http_request.trim_end().as_bytes())
        .expect("https connection failed");

    tls.conn
        .negotiated_cipher_suite()
        .expect("cipher suite failed");

    let mut plaintext = Vec::new();

    tls.read_to_end(&mut plaintext)
        .expect("failed to read from https buffer");

    File::create("foo.txt")
        .expect("file creation failed")
        .write_all(&plaintext)
        .expect("writing to file failed")
}

fn build_request() -> String {
    let mut base_request = [
        "POST /api/elden-ring-nightreign/v1/graphql/query HTTP/1.1",
        "Host: mobalytics.gg",
        "User-Agent: Wget/1.25.0",
        "Accept: */*",
        "Accept-Encoding: identity",
        "Connection: close",
        "Content-Type: application/json",
        "",
        "",
        "",
    ];

    let mut graphql_part = include_str!("../res/Weapons.graphql").to_string();
    graphql_part.retain(|c| !c.is_control());
    let json_part = format!(
        "{{\"variables\":{{\"input\":{{\"staticDataTypes\":[\"weapons\"]}}}},\"query\":\"{gq}\"}}",
        gq = graphql_part
    );
    let con_len = format!("Content-Length: {}", json_part.len());
    base_request[7] = &con_len;
    base_request[9] = &json_part;
    base_request.join("\r\n")
}
