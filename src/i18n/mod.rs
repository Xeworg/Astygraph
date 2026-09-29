//! Internationalization (i18n) module.
//!
//! Provides locale-aware string lookup for the UI. All user-visible text
//! is retrieved through this module — no hardcoded strings in widgets.
//!
//! Locale fallback: if a key is missing from the current locale, the key
//! itself is returned (silently, no panic).

use std::borrow::Cow;

// ---------------------------------------------------------------------------
// Locale
// ---------------------------------------------------------------------------

/// Supported UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    #[default]
    En,
    Es,
}

impl Locale {
    /// Return the next locale in the toggle cycle (En <-> Es).
    #[must_use]
    pub fn toggle(self) -> Self {
        match self {
            Locale::En => Locale::Es,
            Locale::Es => Locale::En,
        }
    }

    /// Short human-readable label for the locale switcher.
    pub fn label(self) -> &'static str {
        match self {
            Locale::En => "EN",
            Locale::Es => "ES",
        }
    }
}

// ---------------------------------------------------------------------------
// Translation storage
// ---------------------------------------------------------------------------

/// A single translation entry: key → value.
type Entry = (&'static str, &'static str);

/// All translation keys and their values for a single locale.
trait Translations {
    fn entries() -> &'static [Entry];
}

/// English (US) translations.
struct En;
impl Translations for En {
    fn entries() -> &'static [Entry] {
        &[
            ("app.title", "Astynex"),
            ("folder.open", "Open Folder"),
            ("folder.open_title", "Open Project Folder"),
            ("folder.close", "Close Folder"),
            ("folder.opening", "Opening folder…"),
            ("explorer.title", "Explorer"),
            ("explorer.up", "Up"),
            ("explorer.discovery_incomplete", "[!] Discovery incomplete — 50,000-entry limit reached."),
            ("explorer.parent_dir", ".."),
            ("source.title", "Source"),
            ("source.select_file", "Select a file to view its contents."),
            ("source.empty_file", "(empty file)"),
            ("source.large_file", "This file is too large for structural analysis (>1 MiB). The text is still visible, but symbol detection and graph features are not available for this file."),
            ("source.error", "Error: {0}"),
            ("source.io_error", "I/O error: {0}"),
            ("source.utf8_error", "Invalid UTF-8: the file contains bytes that are not valid UTF-8 encoding."),
            ("locale.toggle", "Language"),
        ]
    }
}

/// Spanish (Latin American) translations.
struct Es;
impl Translations for Es {
    fn entries() -> &'static [Entry] {
        &[
            ("app.title", "Astynex"),
            ("folder.open", "Abrir carpeta"),
            ("folder.open_title", "Abrir carpeta del proyecto"),
            ("folder.close", "Cerrar carpeta"),
            ("folder.opening", "Abriendo carpeta…"),
            ("explorer.title", "Explorador"),
            ("explorer.up", "Subir"),
            ("explorer.discovery_incomplete", "[!] Descubrimiento incompleto — se alcanzó el límite de 50.000 entradas."),
            ("explorer.parent_dir", ".."),
            ("source.title", "Código"),
            ("source.select_file", "Seleccioná un archivo para ver su contenido."),
            ("source.empty_file", "(archivo vacío)"),
            ("source.large_file", "Este archivo es demasiado grande para análisis estructural (>1 MiB). El texto sigue siendo visible, pero la detección de símbolos y las funciones de grafo no están disponibles para este archivo."),
            ("source.error", "Error: {0}"),
            ("source.io_error", "Error de E/S: {0}"),
            ("source.utf8_error", "UTF-8 inválido: el archivo contiene bytes que no son codificación UTF-8 válida."),
            ("locale.toggle", "Idioma"),
        ]
    }
}

// ---------------------------------------------------------------------------
// I18n
// ---------------------------------------------------------------------------

/// Internationalization context. Holds the active locale and its lookup table.
#[derive(Debug, Clone)]
pub struct I18n {
    locale: Locale,
    /// Sorted by key for O(log n) binary search.
    entries: Vec<(&'static str, &'static str)>,
}

impl I18n {
    /// Build an i18n context for the given locale.
    pub fn new(locale: Locale) -> Self {
        let mut entries = match locale {
            Locale::En => En::entries().to_vec(),
            Locale::Es => Es::entries().to_vec(),
        };
        // Sort for binary search.
        entries.sort_by(|a, b| a.0.cmp(b.0));
        Self { locale, entries }
    }

    /// Look up a translation by key.
    ///
    /// Returns the value if found. If the key is missing, returns the key
    /// itself (silent fallback — no panic).
    ///
    /// Arguments in the form `{0}`, `{1}`, … are replaced by the supplied
    /// `args`. If fewer args are provided than placeholders, remaining
    /// placeholders are left as-is.
    pub fn t(&self, key: &str) -> Cow<'_, str> {
        match self.entries.binary_search_by(|e| e.0.cmp(key)) {
            Ok(idx) => Cow::Borrowed(self.entries[idx].1),
            // Fallback: key itself, owned so it satisfies any lifetime.
            Err(_) => Cow::Owned(key.to_string()),
        }
    }

    /// Like [`I18n::t`] but with positional argument substitution.
    ///
    /// Replaces `{0}`, `{1}`, … with the corresponding entries in `args`.
    pub fn t_args(&self, key: &str, args: &[&str]) -> Cow<'_, str> {
        let template = self.t(key);
        let mut result = template.into_owned();
        for (i, arg) in args.iter().enumerate() {
            let placeholder = format!("{{{}}}", i);
            result = result.replace(&placeholder, arg);
        }
        Cow::Owned(result)
    }

    /// Current locale.
    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// All registered keys in the current locale.
    pub fn keys(&self) -> &[(&'static str, &'static str)] {
        &self.entries
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn all_keys() -> Vec<&'static str> {
        vec![
            "app.title",
            "folder.open",
            "folder.open_title",
            "folder.close",
            "folder.opening",
            "explorer.title",
            "explorer.up",
            "explorer.discovery_incomplete",
            "explorer.parent_dir",
            "source.title",
            "source.select_file",
            "source.empty_file",
            "source.large_file",
            "source.error",
            "source.io_error",
            "source.utf8_error",
            "locale.toggle",
        ]
    }

    #[test]
    fn every_key_present_in_both_locales() {
        let en = I18n::new(Locale::En);
        let es = I18n::new(Locale::Es);
        for key in all_keys() {
            assert!(en.t(key) != key, "key '{}' missing from English", key);
            assert!(es.t(key) != key, "key '{}' missing from Spanish", key);
        }
    }

    #[test]
    fn locale_toggle_cycles() {
        assert_eq!(Locale::En.toggle(), Locale::Es);
        assert_eq!(Locale::Es.toggle(), Locale::En);
    }

    #[test]
    fn locale_labels() {
        assert_eq!(Locale::En.label(), "EN");
        assert_eq!(Locale::Es.label(), "ES");
    }

    #[test]
    fn missing_key_returns_key_itself() {
        let i18n = I18n::new(Locale::En);
        assert_eq!(i18n.t("does.not.exist"), "does.not.exist");
        assert_eq!(i18n.t(""), "");
    }

    #[test]
    fn t_args_replaces_placeholders() {
        let i18n = I18n::new(Locale::En);
        let result = i18n.t_args("source.error", &["file not found"]);
        assert_eq!(result, "Error: file not found");
    }

    #[test]
    fn t_args_ignores_extra_args() {
        let i18n = I18n::new(Locale::En);
        let result = i18n.t_args("source.error", &["oops", "extra"]);
        assert_eq!(result, "Error: oops");
    }

    #[test]
    fn t_args_preserves_unmatched_placeholders() {
        let i18n = I18n::new(Locale::En);
        let result = i18n.t_args("source.error", &[]);
        assert_eq!(result, "Error: {0}");
    }

    #[test]
    fn english_and_spanish_differ_for_contrastable_keys() {
        let en = I18n::new(Locale::En);
        let es = I18n::new(Locale::Es);
        // These keys have intentionally different translations.
        let differing = [
            "folder.open",
            "folder.close",
            "explorer.title",
            "explorer.up",
            "source.title",
            "source.select_file",
        ];
        for key in differing {
            assert_ne!(
                en.t(key).as_ref(),
                es.t(key).as_ref(),
                "key '{}' should differ between locales",
                key
            );
        }
    }

    #[test]
    fn app_title_is_same_in_both_locales() {
        let en = I18n::new(Locale::En);
        let es = I18n::new(Locale::Es);
        assert_eq!(en.t("app.title"), "Astynex");
        assert_eq!(es.t("app.title"), "Astynex");
    }
}
