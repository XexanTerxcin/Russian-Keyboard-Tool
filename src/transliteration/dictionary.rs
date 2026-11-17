use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub const EMBEDDED_WORDS: &str = include_str!("../../resources/dictionaries/russian_words.txt");

#[derive(Debug, Clone)]
pub struct Dictionary {
    words: Vec<String>,
    user_words: HashSet<String>,
}

impl Default for Dictionary {
    fn default() -> Self {
        let mut words: Vec<String> = EMBEDDED_WORDS
            .lines()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        words.sort();
        words.dedup();
        Self {
            words,
            user_words: HashSet::new(),
        }
    }
}

impl Dictionary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_user_dictionary(&mut self, path: &Path) {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                let w = line.trim().to_lowercase();
                if !w.is_empty() {
                    self.add_user_word(&w);
                }
            }
        }
    }

    pub fn save_user_dictionary(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut words: Vec<&String> = self.user_words.iter().collect();
        words.sort();
        let content = words
            .into_iter()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join("\n");
        fs::write(path, content)
    }

    pub fn add_user_word(&mut self, word: &str) {
        let w = word.trim().to_lowercase();
        if !w.is_empty() && !self.user_words.contains(&w) {
            self.user_words.insert(w.clone());
            if let Err(idx) = self.words.binary_search(&w) {
                self.words.insert(idx, w);
            }
        }
    }

    pub fn find_prefix_matches(&self, prefix: &str, limit: usize) -> Vec<String> {
        let p = prefix.trim().to_lowercase();
        if p.is_empty() {
            return Vec::new();
        }

        let start_idx = match self.words.binary_search(&p) {
            Ok(idx) => idx,
            Err(idx) => idx,
        };

        let mut matches = Vec::new();
        for word in &self.words[start_idx..] {
            if word.starts_with(&p) {
                matches.push(word.clone());
                if matches.len() >= limit {
                    break;
                }
            } else {
                break;
            }
        }

        matches.sort_by(|a, b| {
            let a_user = self.user_words.contains(a);
            let b_user = self.user_words.contains(b);
            b_user.cmp(&a_user).then_with(|| a.len().cmp(&b.len()))
        });

        matches
    }
}
