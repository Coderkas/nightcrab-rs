use serde_json::Value;

pub struct Weapon<'a> {
    pub name: &'a str,
    pub range: Option<u8>,
    pub passive: Option<&'a str>,
    pub kind: &'a str,
    pub attack_affinity: Option<&'a str>,
    pub attack_power: [ElementTypes; 6],
    pub guarded_negation: [ElementTypes; 6],
    pub scaling: [Option<Attribute>; 8],
    pub status_ailment: Option<StatusAilment>,
    pub active: Option<&'a str>,
}

pub enum ElementTypes {
    Physical(u8),
    Magic(u8),
    Fire(u8),
    Lightning(u8),
    Holy(u8),
    Boost(u8),
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
    Poison(u8),
    ScarletRot(u8),
    BloodLoss(u8),
    Frostbite(u8),
    Sleep(u8),
    Madness(u8),
    DeathBlight(u8),
}

pub fn parse_weapon_data(weapon_data: &Value) -> Weapon {
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
        passive: get_node_name(weapon_data, "weaponPassive"),
        kind: weapon_data["weaponType"]["name"]
            .as_str()
            .expect("Weapon type was empty"),
        attack_affinity: get_node_name(weapon_data, "attackAffinity"),
        attack_power: [
            ElementTypes::Physical(get_element_val(&weapon_data["attackPower"], 0)),
            ElementTypes::Magic(get_element_val(&weapon_data["attackPower"], 1)),
            ElementTypes::Fire(get_element_val(&weapon_data["attackPower"], 2)),
            ElementTypes::Lightning(get_element_val(&weapon_data["attackPower"], 3)),
            ElementTypes::Holy(get_element_val(&weapon_data["attackPower"], 4)),
            ElementTypes::Boost(get_element_val(&weapon_data["attackPower"], 5)),
        ],
        guarded_negation: [
            ElementTypes::Physical(get_element_val(&weapon_data["guardedNegation"], 0)),
            ElementTypes::Magic(get_element_val(&weapon_data["guardedNegation"], 1)),
            ElementTypes::Fire(get_element_val(&weapon_data["guardedNegation"], 2)),
            ElementTypes::Lightning(get_element_val(&weapon_data["guardedNegation"], 3)),
            ElementTypes::Holy(get_element_val(&weapon_data["guardedNegation"], 4)),
            ElementTypes::Boost(get_element_val(&weapon_data["guardedNegation"], 5)),
        ],
        scaling: parse_scalings(weapon_data),
        status_ailment: match weapon_data["statusAilment"]["value"].is_null() {
            true => None,
            false => Some(get_ailment(weapon_data)),
        },
        active: get_node_name(weapon_data, "ashOfWar"),
    }
}

fn get_node_name<'a>(json_result: &'a Value, node_name: &str) -> Option<&'a str> {
    let err_str = format!(
        "Node was not empty but value was? For object: {}",
        json_result
    );
    match json_result[node_name].is_null() {
        true => None,
        false => Some(
            json_result[node_name]["name"]
                .as_str()
                .expect(err_str.as_str()),
        ),
    }
}

fn get_element_val(json_result: &Value, value_index: usize) -> u8 {
    if json_result[value_index]["value"].is_null() {
        0
    } else {
        json_result[value_index]["value"]
            .as_u64()
            .expect("value was empty or coulnt be parsed into u64") as u8
    }
}

fn get_ailment(json_result: &Value) -> StatusAilment {
    let value = json_result["statusAilment"]["value"]
        .as_u64()
        .expect("failed to parse ailment value") as u8;
    match json_result["statusAilment"]["statusAilmentType"]["name"]
        .as_str()
        .expect("failed to parse ailment type")
    {
        "Poison" => StatusAilment::Poison(value),
        "Scarlet Rot" => StatusAilment::ScarletRot(value),
        "Blood Loss" => StatusAilment::BloodLoss(value),
        "Frostbite" => StatusAilment::Frostbite(value),
        "Sleep" => StatusAilment::Sleep(value),
        "Madness" => StatusAilment::Madness(value),
        "Death Blight" => StatusAilment::DeathBlight(value),
        _ => panic!("unknown status ailment was found while parsing weapon"),
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
