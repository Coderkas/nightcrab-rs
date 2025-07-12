use std::{
    env,
    fs::OpenOptions,
    io::{BufRead, Write},
    net::TcpStream,
    path::Path,
    sync::Arc,
};

use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned, pki_types::ServerName};
use webpki_roots::TLS_SERVER_ROOTS;

struct Weapon {
    name: &'static str,
    range: u8,
    passive: &'static str,
    kind: &'static str,
    attack_affinity: &'static str,
    attack_power: ElementTypes,
    guarded_negation: ElementTypes,
    scaling: Attributes,
    status_ailment: StatusAilment,
    active: &'static str,
}

struct ElementTypes {
    physical: u8,
    magic: u8,
    fire: u8,
    lightning: u8,
    holy: u8,
    boost: u8,
}

struct Attributes {
    vigor: char,
    mind: char,
    endurance: char,
    strength: char,
    dexterity: char,
    intelligence: char,
    faith: char,
    arcane: char,
}

struct StatusAilment {
    kind: &'static str,
    value: u8,
}

fn main() {
    if let Some(arg) = env::args().next() {
        if arg == "run" {
            match Path::new("../weapons.json").try_exists() {
                Ok(true) => println!(""),
                Ok(false) => {
                    println!("weapons.json doesnt exist. Run 'nightcrab update' first")
                }
                Err(err) => println!(
                    "Failed to resolve path to needed data, either the file doesnt exist or there were some other issues"
                ),
            };
        } else if arg == "update" {
            OpenOptions::new()
                .write(true)
                .create(true)
                .open("weapons.json")
                .expect("File");
            send_web_request();
        } else {
            println!(
                "Unknown argument '{}' provided. Possible options are 'run', 'update'",
                arg
            )
        }
    } else {
        println!("Option missing, available parameters are 'run', 'update'")
    }
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

    let mut plaintext = String::new();
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("weapons.json")
        .expect("file creation failed");

    let mut content_start = false;
    loop {
        let content_line = match tls.read_line(&mut plaintext) {
            Ok(0) => break,
            Ok(buf_size) => buf_size > 6,
            Err(err) => {
                println!("{}", err);
                break;
            }
        };

        if plaintext.starts_with("{") {
            content_start = true;
        }

        plaintext.truncate(plaintext.len() - 2);
        if content_start && content_line {
            file.write(plaintext.as_bytes()).expect("error");
        } else {
            println!("{}", plaintext)
        }
        plaintext.clear();
    }
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
