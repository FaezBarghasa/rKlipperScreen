use gettext::Catalog;
use std::fs::File;
use std::path::Path;
use std::sync::RwLock;

pub struct Translator {
    catalog: Option<Catalog>,
}

impl Translator {
    pub fn new(lang: &str) -> Self {
        let lang_code = if lang == "system_lang" {
            // Retrieve system language if configured as system_lang
            std::env::var("LANG")
                .unwrap_or_else(|_| "en".to_string())
                .split('.')
                .next()
                .unwrap_or("en")
                .split('_')
                .next()
                .unwrap_or("en")
                .to_string()
        } else {
            lang.to_string()
        };

        let mo_path = format!("ks_includes/locales/{}/LC_MESSAGES/KlipperScreen.mo", lang_code);
        let catalog = if Path::new(&mo_path).exists() {
            if let Ok(file) = File::open(&mo_path) {
                Catalog::parse(file).ok()
            } else {
                None
            }
        } else {
            None
        };

        Self { catalog }
    }

    pub fn translate(&self, text: &str) -> String {
        if let Some(ref cat) = self.catalog {
            cat.gettext(text).to_string()
        } else {
            text.to_string()
        }
    }
}

// Global thread-safe translator instance
pub static TRANSLATOR: RwLock<Option<Translator>> = RwLock::new(None);

pub fn init_global_translator(lang: &str) {
    let mut translator = TRANSLATOR.write().unwrap();
    *translator = Some(Translator::new(lang));
}

pub fn get_translation(text: &str) -> String {
    let translator = TRANSLATOR.read().unwrap();
    if let Some(ref trans) = *translator {
        trans.translate(text)
    } else {
        text.to_string()
    }
}
