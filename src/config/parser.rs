use ini::Ini;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

#[derive(Clone, Debug)]
pub struct PrinterConfig {
    pub name: String,
    pub moonraker_host: String,
    pub moonraker_port: u16,
    pub moonraker_api_key: String,
}

#[derive(Clone, Debug)]
pub struct PreheatPreset {
    pub name: String,
    pub bed: f32,
    pub extruder: f32,
}

#[derive(Clone, Debug)]
pub struct KConfig {
    pub language: String,
    pub theme: String,
    pub printers: HashMap<String, PrinterConfig>,
    pub preheat_presets: HashMap<String, PreheatPreset>,
    pub raw_ini: Ini,
}

impl KConfig {
    pub fn load_cascading(config_path_override: Option<&str>) -> Self {
        let mut combined_ini = Ini::new();

        // 1. Hardcoded internal fallbacks
        combined_ini.with_section(Some("main"))
            .set("language", "en")
            .set("theme", "z-bolt")
            .set("use_dpms", "true");

        // 2. Built-in defaults (mimicking defaults.conf and its includes)
        let defaults_path = Path::new("config/defaults.conf");
        if defaults_path.exists() {
            info!("Loading default config from config/defaults.conf");
            if let Ok(defaults_ini) = Self::load_file_with_includes(defaults_path) {
                Self::merge_ini(&mut combined_ini, &defaults_ini);
            }
        } else {
            warn!("Default config file config/defaults.conf not found. Using internal fallbacks.");
        }

        // 3. User-defined config path resolution
        let user_config_path = if let Some(path_str) = config_path_override {
            Some(PathBuf::from(path_str))
        } else {
            Self::find_user_config_path()
        };

        // 4. Parse user-defined configuration and merge
        if let Some(ref path) = user_config_path {
            if path.exists() {
                info!("Loading user config from {:?}", path);
                if let Ok(user_ini) = Self::load_file_with_includes(path) {
                    Self::merge_ini(&mut combined_ini, &user_ini);
                }
            } else {
                warn!("Config file {:?} does not exist. Skipping user overrides.", path);
            }
        }

        // Extract sections to structured objects
        let main_sec = combined_ini.section(Some("main"));
        let language = main_sec
            .and_then(|s| s.get("language"))
            .unwrap_or("en")
            .to_string();
        let theme = main_sec
            .and_then(|s| s.get("theme"))
            .unwrap_or("z-bolt")
            .to_string();

        let mut printers = HashMap::new();
        let mut preheat_presets = HashMap::new();

        for (sec, prop) in &combined_ini {
            if let Some(sec_name) = sec {
                if sec_name.starts_with("printer ") {
                    let printer_name = sec_name["printer ".len()..].to_string();
                    let host = prop.get("moonraker_host").unwrap_or("127.0.0.1").to_string();
                    let port = prop.get("moonraker_port")
                        .and_then(|p| p.parse::<u16>().ok())
                        .unwrap_or(7125);
                    let api_key = prop.get("moonraker_api_key").unwrap_or("").to_string();

                    printers.insert(
                        printer_name.clone(),
                        PrinterConfig {
                            name: printer_name,
                            moonraker_host: host,
                            moonraker_port: port,
                            moonraker_api_key: api_key,
                        },
                    );
                } else if sec_name.starts_with("preheat ") {
                    let preset_name = sec_name["preheat ".len()..].to_string();
                    let bed = prop.get("bed")
                        .and_then(|v| v.parse::<f32>().ok())
                        .unwrap_or(0.0);
                    let extruder = prop.get("extruder")
                        .and_then(|v| v.parse::<f32>().ok())
                        .unwrap_or(0.0);

                    preheat_presets.insert(
                        preset_name.clone(),
                        PreheatPreset {
                            name: preset_name,
                            bed,
                            extruder,
                        },
                    );
                }
            }
        }

        // If no printers configured, set a default printer instance
        if printers.is_empty() {
            printers.insert(
                "default".to_string(),
                PrinterConfig {
                    name: "default".to_string(),
                    moonraker_host: "127.0.0.1".to_string(),
                    moonraker_port: 7125,
                    moonraker_api_key: "".to_string(),
                },
            );
        }

        KConfig {
            language,
            theme,
            printers,
            preheat_presets,
            raw_ini: combined_ini,
        }
    }

    fn merge_ini(dest: &mut Ini, src: &Ini) {
        for (sec, prop) in src {
            let section_key = sec.as_deref();
            for (key, val) in prop {
                dest.set_to(section_key, key.to_string(), val.to_string());
            }
        }
    }

    fn load_file_with_includes(path: &Path) -> Result<Ini, ini::Error> {
        let ini = Ini::load_from_file(path)?;
        let mut final_ini = Ini::new();
        Self::merge_ini(&mut final_ini, &ini);

        // Find parent dir to resolve relative includes
        let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));

        for (sec, _prop) in &ini {
            if let Some(sec_name) = sec {
                // Handle sections like [include other.conf] or [include_config other.conf]
                if sec_name.starts_with("include ") || sec_name.starts_with("[include ") {
                    let include_raw = if sec_name.starts_with("include ") {
                        &sec_name["include ".len()..]
                    } else {
                        let inner = &sec_name["[include ".len()..];
                        inner.strip_suffix(']').unwrap_or(inner)
                    };

                    let include_path = parent_dir.join(include_raw.trim());
                    if include_path.exists() {
                        if let Ok(included_ini) = Self::load_file_with_includes(&include_path) {
                            Self::merge_ini(&mut final_ini, &included_ini);
                        }
                    }
                }
            }
        }

        Ok(final_ini)
    }

    fn find_user_config_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/pi".to_string());
        
        let paths = vec![
            format!("{}/printer_data/config/KlipperScreen.conf", home),
            format!("{}/.config/KlipperScreen/KlipperScreen.conf", home),
            format!("{}/KlipperScreen.conf", home),
            "./KlipperScreen.conf".to_string(),
        ];

        for path_str in paths {
            let path = PathBuf::from(path_str);
            if path.exists() {
                return Some(path);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_ini() {
        let mut dest = Ini::new();
        dest.set_to(Some("main"), "language".to_string(), "fr".to_string());
        
        let mut src = Ini::new();
        src.set_to(Some("main"), "theme".to_string(), "material".to_string());
        
        KConfig::merge_ini(&mut dest, &src);
        
        assert_eq!(dest.section(Some("main")).unwrap().get("language").unwrap(), "fr");
        assert_eq!(dest.section(Some("main")).unwrap().get("theme").unwrap(), "material");
    }

    #[test]
    fn test_load_cascading_fallback() {
        let config = KConfig::load_cascading(Some("non_existent_file.conf"));
        assert_eq!(config.language, "en");
        assert_eq!(config.theme, "z-bolt");
        assert!(!config.printers.is_empty());
    }
}

