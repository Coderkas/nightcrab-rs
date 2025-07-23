use serde_json::Value;

pub struct Weapon {
    pub name: String,
    pub range: Option<u8>,
    pub passive: Option<String>,
    pub kind: String,
    pub attack_affinity: String,
    pub attack_power: ElementTypes,
    pub guarded_negation: ElementTypes,
    pub scaling: [Option<Attribute>; 8],
    pub status_ailment: Option<StatusAilment>,
    pub active: String,
}

pub struct ElementTypes {
    pub physical: String,
    pub magic: String,
    pub fire: String,
    pub lightning: String,
    pub holy: String,
    pub boost: String,
}

pub enum Attribute {
    Vigor(usize),
    Mind(usize),
    Endurance(usize),
    Strength(usize),
    Dexterity(usize),
    Intelligence(usize),
    Faith(usize),
    Arcane(usize),
}

pub enum StatusAilment {
    Poison(String),
    ScarletRot(String),
    BloodLoss(String),
    Frostbite(String),
    Sleep(String),
    Madness(String),
    DeathBlight(String),
}

pub fn parse_weapon_data(weapon_data: &Value) -> Weapon {
    Weapon {
        name: weapon_data["name"]
            .as_str()
            .expect("Weapon name was empty, wft?")
            .to_owned(),
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
                    .expect("Weapon passive was not empty, but name was empty?")
                    .to_owned(),
            ),
            true => None,
        },
        kind: weapon_data["weaponType"]["name"]
            .as_str()
            .expect("Weapon type was empty")
            .to_owned(),
        attack_affinity: match weapon_data["attackAffinity"].is_null() {
            false => weapon_data["attackAffinity"]["name"]
                .as_str()
                .expect("attack affinity was empty")
                .to_owned(),
            true => "Unknown".to_owned(),
        },
        attack_power: parse_element_types(&weapon_data["attackPower"]),
        guarded_negation: parse_element_types(&weapon_data["guardedNegation"]),
        scaling: parse_scalings(weapon_data),
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
                            .expect("failed to parse ailment value")
                            .to_string(),
                    ),
                    "Scarlet Rot" => StatusAilment::ScarletRot(
                        weapon_data["statusAilment"]["value"]
                            .as_u64()
                            .expect("failed to parse ailment value")
                            .to_string(),
                    ),
                    "Blood Loss" => StatusAilment::BloodLoss(
                        weapon_data["statusAilment"]["value"]
                            .as_u64()
                            .expect("failed to parse ailment value")
                            .to_string(),
                    ),
                    "Frostbite" => StatusAilment::Frostbite(
                        weapon_data["statusAilment"]["value"]
                            .as_u64()
                            .expect("failed to parse ailment value")
                            .to_string(),
                    ),
                    "Sleep" => StatusAilment::Sleep(
                        weapon_data["statusAilment"]["value"]
                            .as_u64()
                            .expect("failed to parse ailment value")
                            .to_string(),
                    ),
                    "Madness" => StatusAilment::Madness(
                        weapon_data["statusAilment"]["value"]
                            .as_u64()
                            .expect("failed to parse ailment value")
                            .to_string(),
                    ),
                    "Death Blight" => StatusAilment::DeathBlight(
                        weapon_data["statusAilment"]["value"]
                            .as_u64()
                            .expect("failed to parse ailment value")
                            .to_string(),
                    ),
                    _ => panic!("unknown status ailment was found while parsing weapon"),
                },
            ),
        },
        active: match weapon_data["ashOfWar"].is_null() {
            false => weapon_data["ashOfWar"]["name"]
                .as_str()
                .expect(format!("couldnt parse active skill for: {}", weapon_data).as_str())
                .to_owned(),
            true => "Unknown".to_owned(),
        },
    }
}

fn parse_element_types(json_result: &Value) -> ElementTypes {
    let type_value = |value_index: usize| {
        if json_result[value_index]["value"].is_null() {
            String::from("N/A")
        } else {
            json_result[value_index]["value"]
                .as_u64()
                .expect("value was empty or coulnt be parsed into u64")
                .to_string()
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

fn parse_scalings(json_result: &Value) -> [Option<Attribute>; 8] {
    let mut scalings = [None, None, None, None, None, None, None, None];

    for scaling in json_result["attributeScaling"]
        .as_array()
        .expect("failed to parse scalings as array")
        .iter()
    {
        let attribute_value: usize = match scaling["value"]
            .as_str()
            .expect("failed to parse attribute value as str")
        {
            "S" => 0,
            "A" => 1,
            "B" => 2,
            "C" => 3,
            "D" => 4,
            "E" => 5,
            "F" => 6,
            _ => panic!("found weird attribute value"),
        };

        match scaling["attribute"]["name"]
            .as_str()
            .expect("failed to parse attribute name as string")
        {
            "Vigor" => scalings[0] = Some(Attribute::Vigor(attribute_value)),
            "Mind" => scalings[1] = Some(Attribute::Mind(attribute_value)),
            "Endurance" => scalings[2] = Some(Attribute::Endurance(attribute_value)),
            "Strength" => scalings[3] = Some(Attribute::Strength(attribute_value)),
            "Dexterity" => scalings[4] = Some(Attribute::Dexterity(attribute_value)),
            "Intelligence" => scalings[5] = Some(Attribute::Intelligence(attribute_value)),
            "Faith" => scalings[6] = Some(Attribute::Faith(attribute_value)),
            "Arcane" => scalings[7] = Some(Attribute::Arcane(attribute_value)),
            _ => panic!("found weird attribute"),
        }
    }

    scalings
}
