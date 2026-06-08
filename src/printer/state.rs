use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub struct PrinterState {
    pub printer_state: String,
    pub extruder_temp: f32,
    pub extruder_target: f32,
    pub bed_temp: f32,
    pub bed_target: f32,
    pub fan_speed: f32,
    pub print_progress: f32,
    pub print_filename: String,
    pub raw_state: Value,
}

impl PrinterState {
    pub fn new() -> Self {
        Self {
            printer_state: "disconnected".to_string(),
            extruder_temp: 0.0,
            extruder_target: 0.0,
            bed_temp: 0.0,
            bed_target: 0.0,
            fan_speed: 0.0,
            print_progress: 0.0,
            print_filename: String::new(),
            raw_state: serde_json::json!({
                "interpreter_state": "idle",
                "temperature_devices": { "count": 1 },
                "extruders": { "count": 1 },
                "fans": { "count": 1 },
                "leds": { "count": 0 },
                "output_pins": { "count": 0 },
                "pwm_tools": { "count": 0 },
                "config_sections": []
            }),
        }
    }
}

pub fn merge_json(dest: &mut Value, src: &Value) {
    if let (Some(dest_obj), Some(src_obj)) = (dest.as_object_mut(), src.as_object()) {
        for (key, val) in src_obj {
            if val.is_object() {
                if !dest_obj.contains_key(key) {
                    dest_obj.insert(key.clone(), Value::Object(serde_json::Map::new()));
                }
                merge_json(&mut dest_obj[key], val);
            } else {
                dest_obj.insert(key.clone(), val.clone());
            }
        }
    }
}

pub fn reduce_printer_state(state: &mut PrinterState, update: &Value) {
    // Merge into raw_state for minijinja evaluations
    merge_json(&mut state.raw_state, update);

    if let Some(obj) = update.as_object() {
        if let Some(extruder) = obj.get("extruder") {
            if let Some(temp) = extruder.get("temperature").and_then(|t| t.as_f64()) {
                state.extruder_temp = temp as f32;
            }
            if let Some(target) = extruder.get("target").and_then(|t| t.as_f64()) {
                state.extruder_target = target as f32;
            }
        }
        if let Some(bed) = obj.get("heater_bed") {
            if let Some(temp) = bed.get("temperature").and_then(|t| t.as_f64()) {
                state.bed_temp = temp as f32;
            }
            if let Some(target) = bed.get("target").and_then(|t| t.as_f64()) {
                state.bed_target = target as f32;
            }
        }
        if let Some(fan) = obj.get("fan") {
            if let Some(speed) = fan.get("speed").and_then(|s| s.as_f64()) {
                state.fan_speed = speed as f32;
            }
        }
        if let Some(stats) = obj.get("print_stats") {
            if let Some(progress) = stats.get("progress").and_then(|p| p.as_f64()) {
                state.print_progress = (progress * 100.0) as f32;
            }
            if let Some(filename) = stats.get("filename").and_then(|f| f.as_str()) {
                state.print_filename = filename.to_string();
            }
            if let Some(print_state) = stats.get("state").and_then(|s| s.as_str()) {
                state.printer_state = print_state.to_string();
            }
        }
        if let Some(sdcard) = obj.get("virtual_sdcard") {
            if let Some(progress) = sdcard.get("progress").and_then(|p| p.as_f64()) {
                if state.print_progress == 0.0 {
                    state.print_progress = (progress * 100.0) as f32;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_reduce_printer_state_temperatures() {
        let mut state = PrinterState::new();
        let update = json!({
            "extruder": {
                "temperature": 215.5,
                "target": 220.0
            },
            "heater_bed": {
                "temperature": 55.2,
                "target": 60.0
            }
        });

        reduce_printer_state(&mut state, &update);

        assert_eq!(state.extruder_temp, 215.5);
        assert_eq!(state.extruder_target, 220.0);
        assert_eq!(state.bed_temp, 55.2);
        assert_eq!(state.bed_target, 60.0);
    }

    #[test]
    fn test_merge_json() {
        let mut dest = json!({
            "key1": "val1",
            "nested": {
                "inner1": "old"
            }
        });
        let src = json!({
            "key2": "val2",
            "nested": {
                "inner2": "new"
            }
        });

        merge_json(&mut dest, &src);

        assert_eq!(dest["key1"], "val1");
        assert_eq!(dest["key2"], "val2");
        assert_eq!(dest["nested"]["inner1"], "old");
        assert_eq!(dest["nested"]["inner2"], "new");
    }
}
