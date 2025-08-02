use serde_json::Value;

pub struct Weapon<'a> {
    pub name: &'a str,
    pub passive: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub attack_affinity: Option<&'a str>,
    pub attack_power: [ElementValue; 6],
    pub guarded_negation: [ElementValue; 6],
    pub scaling: [Option<AttributeScaling>; 5],
    pub status_ailment: Option<StatusAilment>,
    pub active: Option<&'a str>,
}

pub struct ElementValue(pub String);

pub struct AttributeScaling(pub usize);

pub enum StatusAilment {
    Poison(String),
    ScarletRot(String),
    BloodLoss(String),
    Frostbite(String),
    Sleep(String),
    Madness(String),
    DeathBlight(String),
}

#[allow(clippy::cast_possible_truncation)]
pub fn parse_weapon_data(weapon_data: &Value) -> Weapon {
    Weapon {
        name: weapon_data["name"]
            .as_str()
            .expect("Weapon name was empty, wft?"),
        passive: get_node_name(weapon_data, "weaponPassive"),
        kind: get_node_name(weapon_data, "weaponType"),
        attack_affinity: get_node_name(weapon_data, "attackAffinity"),
        attack_power: get_element_val(&weapon_data["attackPower"]),
        guarded_negation: get_element_val(&weapon_data["guardedNegation"]),
        scaling: parse_scalings(weapon_data),
        status_ailment: if weapon_data["statusAilment"]["value"].is_null() {
            None
        } else {
            Some(get_ailment(weapon_data))
        },
        active: get_node_name(weapon_data, "ashOfWar"),
    }
}

fn get_node_name<'a>(json_result: &'a Value, node_name: &str) -> Option<&'a str> {
    if json_result[node_name].is_null() {
        None
    } else {
        Some(json_result[node_name]["name"].as_str().unwrap_or_else(|| {
            panic!("Node was not empty but value was? For object: {json_result}")
        }))
    }
}

fn get_element_val(json_result: &Value) -> [ElementValue; 6] {
    let mut elements = [
        ElementValue(String::from("0")),
        ElementValue(String::from("0")),
        ElementValue(String::from("0")),
        ElementValue(String::from("0")),
        ElementValue(String::from("0")),
        ElementValue(String::from("0")),
    ];
    for i in 0..5 {
        if !json_result[i]["value"].is_null() {
            elements[i].0 = json_result[i]["value"]
                .as_u64()
                .expect("value was empty or coulnt be parsed into u64")
                .to_string();
        }
    }
    elements
}

fn get_ailment(json_result: &Value) -> StatusAilment {
    let value = json_result["statusAilment"]["value"]
        .as_u64()
        .expect("failed to parse ailment value")
        .to_string();
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

fn parse_scalings(json_result: &Value) -> [Option<AttributeScaling>; 5] {
    let mut attr_arr = [None, None, None, None, None];

    for scale in json_result["attributeScaling"]
        .as_array()
        .expect("failed to parse scalings as array")
    {
        let attribute_value: usize = match scale["value"]
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

        match scale["attribute"]["name"]
            .as_str()
            .expect("failed to parse attribute name as string")
        {
            "Strength" => attr_arr[0] = Some(AttributeScaling(attribute_value)),
            "Dexterity" => attr_arr[1] = Some(AttributeScaling(attribute_value)),
            "Intelligence" => attr_arr[2] = Some(AttributeScaling(attribute_value)),
            "Faith" => attr_arr[3] = Some(AttributeScaling(attribute_value)),
            "Arcane" => attr_arr[4] = Some(AttributeScaling(attribute_value)),
            _ => panic!("found weird attribute"),
        }
    }

    attr_arr
}
