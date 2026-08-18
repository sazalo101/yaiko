//! Lightweight localization and locale-negotiation primitives.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Locale(String);

impl Locale {
    pub fn parse(value: &str) -> Result<Self, I18nError> {
        let value = value.trim().replace('_', "-");
        if value.is_empty()
            || value.split('-').any(|part| {
                part.is_empty()
                    || !part
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric())
            })
        {
            return Err(I18nError::InvalidLocale);
        }
        Ok(Self(value.to_ascii_lowercase()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    fn language(&self) -> &str {
        self.0.split('-').next().unwrap_or(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum I18nError {
    InvalidLocale,
    MissingKey,
}

#[derive(Debug, Clone, Default)]
pub struct Catalog {
    entries: BTreeMap<String, String>,
}

impl Catalog {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.entries.insert(key.into(), value.into());
        self
    }
    fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }
}

#[derive(Debug, Clone)]
pub struct Translator {
    catalogs: BTreeMap<Locale, Catalog>,
    default_locale: Locale,
}

impl Translator {
    pub fn new(default_locale: Locale) -> Self {
        Self {
            catalogs: BTreeMap::new(),
            default_locale,
        }
    }
    pub fn add_catalog(mut self, locale: Locale, catalog: Catalog) -> Self {
        self.catalogs.insert(locale, catalog);
        self
    }
    pub fn translate(
        &self,
        locale: &Locale,
        key: &str,
        values: &BTreeMap<String, String>,
    ) -> Result<String, I18nError> {
        let template = self
            .catalogs
            .get(locale)
            .and_then(|catalog| catalog.get(key))
            .or_else(|| {
                self.catalogs
                    .get(&self.default_locale)
                    .and_then(|catalog| catalog.get(key))
            })
            .ok_or(I18nError::MissingKey)?;
        Ok(interpolate(template, values))
    }
    pub fn negotiate(&self, accept_language: &str) -> Locale {
        for candidate in accept_language
            .split(',')
            .filter_map(|part| part.split(';').next())
            .filter_map(|value| Locale::parse(value).ok())
        {
            if self.catalogs.contains_key(&candidate) {
                return candidate;
            }
            if let Some(locale) = self
                .catalogs
                .keys()
                .find(|locale| locale.language() == candidate.language())
            {
                return locale.clone();
            }
        }
        self.default_locale.clone()
    }
}

fn interpolate(template: &str, values: &BTreeMap<String, String>) -> String {
    let mut output = template.to_string();
    for (key, value) in values {
        output = output.replace(&format!("{{{{{key}}}}}"), value);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translator() -> Translator {
        let default = Locale::parse("en-US").unwrap();
        Translator::new(default.clone())
            .add_catalog(default, Catalog::new().insert("hello", "Hello, {{name}}!"))
            .add_catalog(
                Locale::parse("fr").unwrap(),
                Catalog::new().insert("hello", "Bonjour, {{name}}!"),
            )
    }

    #[test]
    fn negotiates_exact_language_and_falls_back_to_default() {
        let translator = translator();
        assert_eq!(translator.negotiate("fr-CA, en;q=0.8").as_str(), "fr");
        assert_eq!(translator.negotiate("de").as_str(), "en-us");
    }

    #[test]
    fn interpolates_catalog_values_and_falls_back() {
        let translator = translator();
        let values = BTreeMap::from([(String::from("name"), String::from("Ada"))]);
        assert_eq!(
            translator
                .translate(&Locale::parse("fr").unwrap(), "hello", &values)
                .unwrap(),
            "Bonjour, Ada!"
        );
        assert_eq!(
            translator
                .translate(&Locale::parse("de").unwrap(), "hello", &values)
                .unwrap(),
            "Hello, Ada!"
        );
    }

    #[test]
    fn handles_invalid_locales_and_missing_keys() {
        assert_eq!(Locale::parse(""), Err(I18nError::InvalidLocale));
        let translator = translator();
        assert_eq!(
            translator.translate(&Locale::parse("en").unwrap(), "missing", &BTreeMap::new()),
            Err(I18nError::MissingKey)
        );
    }
}
