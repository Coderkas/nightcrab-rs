use serde_json::Value;

pub struct Weapon<'a> {
    pub name: &'a str,
    pub range: Option<u8>,
    pub passive: Option<&'a str>,
    pub kind: &'a str,
    pub attack_affinity: &'a str,
    pub attack_power: ElementTypes,
    pub guarded_negation: ElementTypes,
    pub scaling: [Attribute<'a>; 4],
    pub status_ailment: Option<StatusAilment>,
    pub active: &'a str,
}

pub struct ElementTypes {
    pub physical: u8,
    pub magic: u8,
    pub fire: u8,
    pub lightning: u8,
    pub holy: u8,
    pub boost: u8,
}

pub enum Attribute<'a> {
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

pub fn parse_weapon_data<'a>(weapon_data: &'a Value) -> Weapon<'a> {
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
