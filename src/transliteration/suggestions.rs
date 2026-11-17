use super::dictionary::Dictionary;
use super::parser::TransliterationParser;

#[derive(Debug, Clone)]
pub struct SuggestionEngine {
    dictionary: Dictionary,
    parser: TransliterationParser,
    enabled: bool,
    max_suggestions: usize,
}

impl Default for SuggestionEngine {
    fn default() -> Self {
        Self {
            dictionary: Dictionary::default(),
            parser: TransliterationParser::default(),
            enabled: true,
            max_suggestions: 7,
        }
    }
}

impl SuggestionEngine {
    pub fn new(dictionary: Dictionary, parser: TransliterationParser) -> Self {
        Self {
            dictionary,
            parser,
            enabled: true,
            max_suggestions: 7,
        }
    }

    pub fn set_auto_capitalize(&mut self, enabled: bool) {
        self.parser.set_auto_capitalize(enabled);
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn dictionary_mut(&mut self) -> &mut Dictionary {
        &mut self.dictionary
    }

    pub fn get_suggestions(&self, input: &str) -> Vec<String> {
        if !self.enabled || input.is_empty() {
            return Vec::new();
        }

        let transliterated = self.parser.transliterate(input);
        let prefix = transliterated.trim();
        if prefix.is_empty() {
            return Vec::new();
        }

        let mut suggestions = self
            .dictionary
            .find_prefix_matches(prefix, self.max_suggestions);

        // Always ensure the exact transliteration is the first candidate if not present
        if !suggestions.iter().any(|s| s.eq_ignore_ascii_case(prefix)) {
            suggestions.insert(0, prefix.to_lowercase());
        }

        // Apply input casing
        suggestions
            .into_iter()
            .map(|word| self.parser.apply_case(&word, input))
            .collect()
    }
}
