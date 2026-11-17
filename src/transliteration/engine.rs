use super::dictionary::Dictionary;
use super::mappings::MappingRule;
use super::parser::TransliterationParser;
use super::suggestions::SuggestionEngine;

#[derive(Debug, Clone)]
pub struct Engine {
    parser: TransliterationParser,
    suggestions: SuggestionEngine,
}

impl Default for Engine {
    fn default() -> Self {
        let parser = TransliterationParser::default();
        let dict = Dictionary::default();
        let suggestions = SuggestionEngine::new(dict, parser.clone());
        Self {
            parser,
            suggestions,
        }
    }
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn transliterate(&self, input: &str) -> String {
        self.parser.transliterate(input)
    }

    pub fn get_suggestions(&self, input: &str) -> Vec<String> {
        self.suggestions.get_suggestions(input)
    }

    pub fn set_mappings(&mut self, rules: Vec<MappingRule>) {
        self.parser.set_rules(rules);
    }

    pub fn set_suggestions_enabled(&mut self, enabled: bool) {
        self.suggestions.set_enabled(enabled);
    }

    pub fn set_auto_capitalize(&mut self, enabled: bool) {
        self.parser.set_auto_capitalize(enabled);
        self.suggestions.set_auto_capitalize(enabled);
    }

    pub fn learn_word(&mut self, word: &str) {
        self.suggestions.dictionary_mut().add_user_word(word);
    }
}
