use crate::config::parser::KConfig;
use minijinja::{context, Environment};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum MenuAction {
    NavigateToPanel(String),
    SubMenu(String), // Path of the submenu, e.g., "__main more"
    ExecuteGcode(String),
    SystemCommand(String),
}

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub name: String,
    pub icon_name: String,
    pub action: MenuAction,
    pub enable_expression: Option<String>,
}

pub struct MenuRouter {
    // Map from parent path (e.g. "__main" or "__main more") to list of menu items
    pub menus: HashMap<String, Vec<MenuItem>>,
    // Breadcrumb navigation stack
    pub navigation_stack: Vec<String>,
}

impl MenuRouter {
    pub fn new(config: &KConfig) -> Self {
        let mut router = Self {
            menus: HashMap::new(),
            navigation_stack: vec!["__main".to_string()],
        };
        router.parse_menus(config);
        router
    }

    fn parse_menus(&mut self, config: &KConfig) {
        // Collect all menu definitions from configuration
        // Sections look like: [menu __main move], [menu __main more bedlevel]
        let mut items_by_parent: HashMap<String, Vec<(String, String, String, Option<String>, Option<String>, Option<String>)>> = HashMap::new();

        for (sec, prop) in &config.raw_ini {
            if let Some(sec_name) = sec {
                if sec_name.starts_with("menu ") {
                    let path_part = sec_name["menu ".len()..].trim();
                    let mut parts: Vec<&str> = path_part.split_whitespace().collect();
                    if parts.is_empty() {
                        continue;
                    }

                    // For example: "__main more bedlevel"
                    // item key is "bedlevel", parent is "__main more"
                    let item_key = parts.pop().unwrap().to_string();
                    let parent_path = parts.join(" ");

                    let name = prop.get("name").unwrap_or(&item_key).to_string();
                    let icon = prop.get("icon").unwrap_or("").to_string();
                    let panel = prop.get("panel").map(|v| v.to_string());
                    let method = prop.get("method").map(|v| v.to_string());
                    let enable = prop.get("enable").map(|v| v.to_string());

                    items_by_parent
                        .entry(parent_path)
                        .or_default()
                        .push((item_key, name, icon, panel, method, enable));
                }
            }
        }

        // Clean up expression syntax for minijinja (e.g. remove {{ }} braces if present)
        let clean_expr = |expr: &str| -> String {
            let trimmed = expr.trim();
            if trimmed.starts_with("{{") && trimmed.ends_with("}}") {
                trimmed[2..trimmed.len() - 2].trim().to_string()
            } else {
                trimmed.to_string()
            }
        };

        // Construct MenuItem trees
        for (parent_path, raw_items) in items_by_parent {
            let mut menu_items = Vec::new();

            for (item_key, name_raw, icon, panel, method, enable) in raw_items {
                // Parse name (remove {{ gettext('...') }})
                let mut name = name_raw.clone();
                if name.contains("gettext") {
                    if let Some(start) = name.find('\'') {
                        if let Some(end) = name[start + 1..].find('\'') {
                            name = name[start + 1..start + 1 + end].to_string();
                        }
                    } else if let Some(start) = name.find('"') {
                        if let Some(end) = name[start + 1..].find('"') {
                            name = name[start + 1..start + 1 + end].to_string();
                        }
                    }
                }

                let enable_expr = enable.map(|e| clean_expr(&e));

                // Determine action
                let action = if let Some(p) = panel {
                    MenuAction::NavigateToPanel(p)
                } else if let Some(m) = method {
                    if m == "ks_confirm_save" {
                        MenuAction::ExecuteGcode("SAVE_CONFIG".to_string())
                    } else {
                        MenuAction::ExecuteGcode(m)
                    }
                } else {
                    // It must be a submenu container
                    let child_path = if parent_path.is_empty() {
                        item_key.clone()
                    } else {
                        format!("{} {}", parent_path, item_key)
                    };
                    MenuAction::SubMenu(child_path)
                };

                menu_items.push(MenuItem {
                    name,
                    icon_name: icon,
                    action,
                    enable_expression: enable_expr,
                });
            }

            self.menus.insert(parent_path, menu_items);
        }
    }

    pub fn get_active_menu_path(&self) -> &str {
        self.navigation_stack.last().map(|s| s.as_str()).unwrap_or("__main")
    }

    pub fn get_active_menu_items(&self, printer_state: &Value) -> Vec<MenuItem> {
        let active_path = self.get_active_menu_path();
        let items = match self.menus.get(active_path) {
            Some(v) => v.clone(),
            None => return Vec::new(),
        };

        // Evaluate enable expressions
        let mut filtered = Vec::new();
        let env = Environment::new();

        for item in items {
            let is_enabled = if let Some(ref expr) = item.enable_expression {
                if expr == "true" {
                    true
                } else if expr == "false" {
                    false
                } else {
                    let context = context! {
                        printer => printer_state,
                        moonraker_connected => true,
                    };
                    env.render_str(&format!("{{{{ {} }}}}", expr), context)
                        .map(|res| res.trim() == "true")
                        .unwrap_or(true)
                }
            } else {
                true
            };

            if is_enabled {
                filtered.push(item);
            }
        }

        filtered
    }

    pub fn navigate_to(&mut self, path: String) {
        self.navigation_stack.push(path);
    }

    pub fn go_back(&mut self) -> bool {
        if self.navigation_stack.len() > 1 {
            self.navigation_stack.pop();
            true
        } else {
            false
        }
    }

    pub fn go_home(&mut self) {
        self.navigation_stack.truncate(1);
    }
}
