use std::{
    env,
    fs::{File, OpenOptions},
    io::{BufRead, Write},
    net::TcpStream,
    sync::Arc,
};

use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned, pki_types::ServerName};
use serde_json::Value;
use webpki_roots::TLS_SERVER_ROOTS;

#[derive(Debug, Deserialize)]
#[serde(rename = "staticDataEntity")]
struct Weapon {
    name: &'static str,
    range: Option<u8>,
    #[serde(rename = "weaponPassive")]
    passive: Option<&'static str>,
    #[serde(rename = "weaponWeapon")]
    kind: &'static str,
    attack_affinity: &'static str,
    attack_power: ElementTypes,
    guarded_negation: ElementTypes,
    scaling: [Attribute; 4],
    status_ailment: Option<StatusAilment>,
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

enum Attribute {
    Vigor(char),
    Mind(char),
    Endurance(char),
    Strength(char),
    Dexterity(char),
    Intelligence(char),
    Faith(char),
    Arcane(char),
}

enum StatusAilment {
    Poison(u8),
    ScarletRot(u8),
    BloodLoss(u8),
    Frostbite(u8),
    Sleep(u8),
    Madness(u8),
    DeathBlight(u8),
}

fn main() {
    if let Some(arg) = env::args().next() {
        if arg == "run" {
            match OpenOptions::new().write(false).open("./weapon.json") {
                Ok(f) => println!("Succeeded in opening file"),
                Err(err) => println!("Failed to open data file because of error: {}", err),
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

fn parse_json(json_file: &File) {
    let json_result: Result<Value, serde_json::Error> = serde_json::from_reader(json_file);
    if let Ok(v) = json_result {
        let weapon_data = &v["data"]["game"]["documents"]["wikiDocuments"]["documents"][0]["data"]
            ["staticDataEntity"];
        let serialized_weapon = Weapon {
            name: weapon_data["name"]
                .as_str()
                .expect("Weapon name was empty, wft?"),
            range: match weapon_data["range"].is_null() {
                true => Some(
                    weapon_data["range"]
                        .as_u64()
                        .expect("Range has a value but it wasnt a number?")
                        as u8,
                ),
                false => None,
            },
        };
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
                println!("{}", err);
                break;
            }
        };

        if string_buf.starts_with("{") {
            content_start = true;
        }

        string_buf.truncate(string_buf.len() - 2);
        if content_start && content_line {
            json_file.write(string_buf.as_bytes()).expect("error");
        } else {
            println!("{}", string_buf)
        }
        string_buf.clear();
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
