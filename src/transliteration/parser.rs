use super::mappings::{default_mappings, MappingRule};

#[derive(Debug, Clone)]
pub struct TransliterationParser {
    rules: Vec<MappingRule>,
    auto_capitalize: bool,
}

impl Default for TransliterationParser {
    fn default() -> Self {
        Self::new(default_mappings())
    }
}

impl TransliterationParser {
    pub fn new(mut rules: Vec<MappingRule>) -> Self {
        rules.sort_by_key(|rule| std::cmp::Reverse(rule.pattern.len()));
        Self {
            rules,
            auto_capitalize: true,
        }
    }

    pub fn set_rules(&mut self, mut rules: Vec<MappingRule>) {
        rules.sort_by_key(|rule| std::cmp::Reverse(rule.pattern.len()));
        self.rules = rules;
    }

    /// When disabled, output stays lowercase regardless of how the input was typed.
    pub fn set_auto_capitalize(&mut self, enabled: bool) {
        self.auto_capitalize = enabled;
    }

    pub fn apply_case(&self, replacement: &str, source: &str) -> String {
        if self.auto_capitalize {
            apply_case(replacement, source)
        } else {
            replacement.to_string()
        }
    }

    pub fn transliterate(&self, input: &str) -> String {
        let mut result = String::new();
        let mut cur_word = String::new();

        for ch in input.chars() {
            if ch.is_ascii_alphabetic() || ch == '\'' || ch == '"' {
                cur_word.push(ch);
            } else {
                if !cur_word.is_empty() {
                    result.push_str(&self.transliterate_word(&cur_word));
                    cur_word.clear();
                }
                result.push(ch);
            }
        }

        if !cur_word.is_empty() {
            result.push_str(&self.transliterate_word(&cur_word));
        }

        result
    }

    pub fn transliterate_word(&self, word: &str) -> String {
        let lower = word.to_lowercase();

        // Exact & contextual words requested by specification
        if lower == "zhizn" {
            return self.apply_case("жизнь", word);
        }
        if lower == "lyubov" {
            return self.apply_case("любовь", word);
        }
        if lower == "den" {
            return self.apply_case("день", word);
        }
        if lower.ends_with("zhizn") && word.len() >= 5 {
            let split = word.len() - 5;
            return self.transliterate_word(&word[..split])
                + &self.apply_case("жизнь", &word[split..]);
        }
        if lower.ends_with("lyubov") && word.len() >= 6 {
            let split = word.len() - 6;
            return self.transliterate_word(&word[..split])
                + &self.apply_case("любовь", &word[split..]);
        }
        if lower.ends_with("den") && word.len() >= 4 {
            let split = word.len() - 3;
            return self.transliterate_word(&word[..split])
                + &self.apply_case("день", &word[split..]);
        }
        if lower.ends_with("zn") && word.len() >= 3 && !lower.ends_with("izn") {
            let split = word.len() - 2;
            return self.transliterate_word(&word[..split])
                + &self.apply_case("знь", &word[split..]);
        }

        let chars: Vec<char> = word.chars().collect();
        let mut i = 0usize;
        let mut out = String::new();

        while i < chars.len() {
            let mut matched = false;
            for rule in &self.rules {
                let pat_len = rule.pattern.chars().count();
                if i + pat_len <= chars.len() {
                    let chunk: String = chars[i..i + pat_len].iter().collect();
                    if chunk.eq_ignore_ascii_case(&rule.pattern) {
                        out.push_str(&self.apply_case(&rule.replacement, &chunk));
                        i += pat_len;
                        matched = true;
                        break;
                    }
                }
            }

            if !matched {
                out.push(chars[i]);
                i += 1;
            }
        }

        out
    }
}

pub fn apply_case(repl: &str, src: &str) -> String {
    let all_upper = src.chars().all(|c| c.is_ascii_uppercase());
    let first_upper = src
        .chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false);

    if all_upper {
        repl.chars()
            .map(|c| c.to_uppercase().to_string())
            .collect::<String>()
    } else if first_upper {
        let mut chars: Vec<char> = repl.chars().collect();
        if let Some(first) = chars.first_mut() {
            *first = first.to_uppercase().next().unwrap_or(*first);
        }
        chars.into_iter().collect()
    } else {
        repl.to_string()
    }
}
