use std::{
    env,
    fs::OpenOptions,
    io::{BufRead, Write},
    net::TcpStream,
    sync::Arc,
};

use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned, pki_types::ServerName};
use serde_json::Value;
use webpki_roots::TLS_SERVER_ROOTS;

struct Weapon<'a> {
    name: &'a str,
    range: Option<u8>,
    passive: Option<&'a str>,
    kind: &'a str,
    attack_affinity: &'a str,
    attack_power: ElementTypes,
    guarded_negation: ElementTypes,
    scaling: [Attribute<'a>; 4],
    status_ailment: Option<StatusAilment>,
    active: &'a str,
}

struct ElementTypes {
    physical: u8,
    magic: u8,
    fire: u8,
    lightning: u8,
    holy: u8,
    boost: u8,
}

enum Attribute<'a> {
    Vigor(&'a str),
    Mind(&'a str),
    Endurance(&'a str),
    Strength(&'a str),
    Dexterity(&'a str),
    Intelligence(&'a str),
    Faith(&'a str),
    Arcane(&'a str),
    Unknown,
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
    if let Some(arg) = env::args().nth(1) {
        println!("{}", arg);
        if arg == "run" {
            match OpenOptions::new()
                .write(false)
                .read(true)
                .open("weapons.json")
            {
                Ok(f) => {
                    let json_values: Result<Value, serde_json::Error> = serde_json::from_reader(f);
                    let weapon_json = &json_values
                        .expect("Failed in parsing json to serde value struct")["data"]["game"]["documents"]
                        ["wikiDocuments"]["documents"];
                    let mut weapon_data = Vec::new();
                    for weapon in weapon_json
                        .as_array()
                        .expect("Arrary of data.staticDataEnity wasnt an array")
                    {
                        weapon_data.push(parse_weapon_data(&weapon["data"]["staticDataEntity"]));
                    }
                    for item in weapon_data {
                        println!("{}", item.name)
                    }
                }
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

fn parse_weapon_data<'a>(weapon_data: &'a Value) -> Weapon<'a> {
    Weapon {
        name: weapon_data["name"]
            .as_str()
            .expect("Weapon name was empty, wft?"),
        range: match weapon_data["range"].is_null() {
            false => Some(
                weapon_data["range"]
                    .as_u64()
                    .expect("Range has a value but it wasnt a number?") as u8,
            ),
            true => None,
        },
        passive: match weapon_data["weaponPassive"].is_null() {
            false => Some(
                weapon_data["weaponPassive"]["name"]
                    .as_str()
                    .expect("Weapon passive was not empty, but name was empty?"),
            ),
            true => None,
        },
        kind: weapon_data["weaponType"]["name"]
            .as_str()
            .expect("Weapon type was empty"),
        attack_affinity: match weapon_data["attackAffinity"].is_null() {
            false => weapon_data["attackAffinity"]["name"]
                .as_str()
                .expect("attack affinity was empty"),
            true => "Unknown",
        },
        attack_power: parse_element_types(&weapon_data["attackPower"]),
        guarded_negation: parse_element_types(&weapon_data["guardedNegation"]),
        scaling: parse_scalings(&weapon_data),
        status_ailment: match weapon_data["statusAilment"]["value"].is_null() {
            true => None,
            false => Some(
                match weapon_data["statusAilment"]["statusAilmentType"]["name"]
                    .as_str()
                    .expect("failed to parse ailment type")
                {
                    "Poison" => StatusAilment::Poison(
                        weapon_data["statusAilment"]["value"]
                            .as_u64()
                            .expect("failed to parse ailment value") as u8,
                    ),
                    "Scarlet Rot" => StatusAilment::ScarletRot(
                        weapon_data["statusAilment"]["value"]
                            .as_u64()
                            .expect("failed to parse ailment value") as u8,
                    ),
                    "Blood Loss" => StatusAilment::BloodLoss(
                        weapon_data["statusAilment"]["value"]
                            .as_u64()
                            .expect("failed to parse ailment value") as u8,
                    ),
                    "Frostbite" => StatusAilment::Frostbite(
                        weapon_data["statusAilment"]["value"]
                            .as_u64()
                            .expect("failed to parse ailment value") as u8,
                    ),
                    "Sleep" => StatusAilment::Sleep(
                        weapon_data["statusAilment"]["value"]
                            .as_u64()
                            .expect("failed to parse ailment value") as u8,
                    ),
                    "Madness" => StatusAilment::Madness(
                        weapon_data["statusAilment"]["value"]
                            .as_u64()
                            .expect("failed to parse ailment value") as u8,
                    ),
                    "Death Blight" => StatusAilment::DeathBlight(
                        weapon_data["statusAilment"]["value"]
                            .as_u64()
                            .expect("failed to parse ailment value") as u8,
                    ),
                    _ => panic!("unknown status ailment was found while parsing weapon"),
                },
            ),
        },
        active: match weapon_data["ashOfWar"].is_null() {
            false => weapon_data["ashOfWar"]["name"]
                .as_str()
                .expect(format!("couldnt parse active skill for: {}", weapon_data).as_str()),
            true => "Unknown",
        },
    }
}

fn parse_element_types(json_result: &Value) -> ElementTypes {
    let type_value = |value_index: usize| {
        if json_result[value_index]["value"].is_null() {
            0
        } else {
            json_result[value_index]["value"]
                .as_u64()
                .expect("value was empty or coulnt be parsed into u64") as u8
        }
    };
    ElementTypes {
        physical: type_value(0),
        magic: type_value(1),
        fire: type_value(2),
        lightning: type_value(3),
        holy: type_value(4),
        boost: type_value(5),
    }
}

fn parse_scalings<'a>(json_result: &'a Value) -> [Attribute<'a>; 4] {
    let mut scalings: [Attribute; 4] = [
        Attribute::Unknown,
        Attribute::Unknown,
        Attribute::Unknown,
        Attribute::Unknown,
    ];
    for (array_index, scaling) in json_result["attributeScaling"]
        .as_array()
        .expect("failed to parse scalings as array")
        .iter()
        .enumerate()
    {
        let attribute_value = scaling["value"]
            .as_str()
            .expect("failed to parse attribute value as str");
        scalings[array_index] = match scaling["attribute"]["name"]
            .as_str()
            .expect("failed to parse attribute name as string")
        {
            "Vigor" => Attribute::Vigor(attribute_value),
            "Mind" => Attribute::Mind(attribute_value),
            "Endurance" => Attribute::Endurance(attribute_value),
            "Strength" => Attribute::Strength(attribute_value),
            "Dexterity" => Attribute::Dexterity(attribute_value),
            "Intelligence" => Attribute::Intelligence(attribute_value),
            "Faith" => Attribute::Faith(attribute_value),
            "Arcane" => Attribute::Arcane(attribute_value),
            _ => panic!("found weird attribute"),
        }
    }

    scalings
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
