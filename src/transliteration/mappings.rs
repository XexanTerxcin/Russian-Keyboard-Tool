use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingRule {
    pub pattern: String,
    pub replacement: String,
}

pub fn default_mappings() -> Vec<MappingRule> {
    let raw = vec![
        // 4 chars
        ("shch", "щ"),
        // 3 chars
        ("sch", "щ"),
        ("shh", "щ"),
        // 2 chars
        ("zh", "ж"),
        ("kh", "х"),
        ("ts", "ц"),
        ("ch", "ч"),
        ("sh", "ш"),
        ("yo", "ё"),
        ("yu", "ю"),
        ("ya", "я"),
        ("ye", "э"),
        ("tc", "ц"),
        ("ju", "ю"),
        ("ja", "я"),
        ("je", "э"),
        ("eh", "э"),
        // 1 char
        ("a", "а"),
        ("b", "б"),
        ("v", "в"),
        ("g", "г"),
        ("d", "д"),
        ("e", "е"),
        ("z", "з"),
        ("i", "и"),
        ("j", "й"),
        ("k", "к"),
        ("l", "л"),
        ("m", "м"),
        ("n", "н"),
        ("o", "о"),
        ("p", "п"),
        ("r", "р"),
        ("s", "с"),
        ("t", "т"),
        ("u", "у"),
        ("f", "ф"),
        ("h", "х"),
        ("c", "ц"),
        ("w", "в"),
        ("x", "х"),
        ("y", "ы"),
        ("'", "ь"),
        ("\"", "ъ"),
        ("q", "я"),
    ];

    raw.into_iter()
        .map(|(p, r)| MappingRule {
            pattern: p.to_string(),
            replacement: r.to_string(),
        })
        .collect()
}
