use serde_json::{json, Value};
use solana_trader_proto::api::Project;

pub fn convert_string_enums(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                match (key.as_str(), &val) {
                    // Project enum conversion
                    ("project", Value::String(s)) => {
                        if let Some(project_enum) = Project::from_str_name(s) {
                            *val = json!(project_enum as i32);
                        }
                    }
                    
                    // String to numeric conversions
                    // Leaving this commented out for now, 
                    // as we try out a new custom deserialization in proto build
                    /*
                    (k, Value::String(s)) if ["tradeFeeRate", "height", "token1Reserves", 
                                            "token2Reserves", "slot", "time", "openTime"]
                                            .contains(&k) => {
                        if let Ok(num) = s.parse::<i64>() {
                            *val = json!(num);
                        }
                    }
                    */
                    
                    // Infinity enum conversion
                    ("infinity", Value::String(s)) => {
                        let mapped = match s.as_str() {
                            "INF_NOT" => 0,
                            "INF" => 1,
                            "INF_NEG" => 2,
                            _ => return,
                        };
                        *val = json!(mapped);
                    }
                    
                    // Recurse for nested structures
                    _ => convert_string_enums(val),
                }
            }
        }
        Value::Array(arr) => arr.iter_mut().for_each(convert_string_enums),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversions() {
        let mut value = json!({
            "project": "P_JUPITER",
            "tradeFeeRate": "1000",
            "nested": {
                "project": "P_RAYDIUM",
                "priceImpactPercent": {
                    "infinity": "INF_NOT"
                }
            },
            "array": [
                {"project": "P_OPENBOOK"}
            ]
        });

        convert_string_enums(&mut value);

        assert_eq!(value["project"], 2);
        assert_eq!(value["tradeFeeRate"], 1000);
        assert_eq!(value["nested"]["project"], 3);
        assert_eq!(value["nested"]["priceImpactPercent"]["infinity"], 0);
        assert_eq!(value["array"][0]["project"], 5);
    }
}