use granit_types::{builtin_themes, Theme, ThemeMeta};

/// In-memory registry of all built-in themes.
pub struct ThemeRegistry {
    themes: Vec<Theme>,
}

impl ThemeRegistry {
    pub fn new() -> Self {
        Self {
            themes: builtin_themes(),
        }
    }

    pub fn list(&self) -> Vec<ThemeMeta> {
        self.themes.iter().map(|t| t.meta()).collect()
    }

    /// Get a theme by id. Falls back to "default" if the id is not found.
    pub fn get(&self, id: &str) -> &Theme {
        self.themes
            .iter()
            .find(|t| t.id == id)
            .unwrap_or_else(|| self.themes.first().expect("at least one theme"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_contains_five_themes() {
        let reg = ThemeRegistry::new();
        let list = reg.list();
        assert_eq!(list.len(), 5);
        let ids: Vec<&str> = list.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"default"));
        assert!(ids.contains(&"latte"));
        assert!(ids.contains(&"frappe"));
        assert!(ids.contains(&"macchiato"));
        assert!(ids.contains(&"mocha"));
    }

    #[test]
    fn test_registry_get_known_theme() {
        let reg = ThemeRegistry::new();
        let mocha = reg.get("mocha");
        assert_eq!(mocha.id, "mocha");
        assert_eq!(mocha.window_bg, "#1e1e2e");
    }

    #[test]
    fn test_registry_get_unknown_falls_back_to_default() {
        let reg = ThemeRegistry::new();
        let t = reg.get("nonexistent");
        assert_eq!(t.id, "default");
    }

    #[test]
    fn test_default_theme_is_dark() {
        let reg = ThemeRegistry::new();
        assert!(reg.get("default").is_dark);
        assert!(!reg.get("latte").is_dark);
    }
}
