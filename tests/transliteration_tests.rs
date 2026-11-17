use russian_keyboard_tool::transliteration::Engine;

#[test]
fn test_required_words() {
    let engine = Engine::new();
    assert_eq!(engine.transliterate("privet"), "привет");
    assert_eq!(engine.transliterate("Privet"), "Привет");
    assert_eq!(engine.transliterate("PRIVET"), "ПРИВЕТ");
    assert_eq!(engine.transliterate("khorosho"), "хорошо");
    assert_eq!(engine.transliterate("spasibo"), "спасибо");
    assert_eq!(engine.transliterate("moskva"), "москва");
    assert_eq!(engine.transliterate("ya"), "я");
    assert_eq!(engine.transliterate("yu"), "ю");
    assert_eq!(engine.transliterate("yo"), "ё");
    assert_eq!(engine.transliterate("zh"), "ж");
    assert_eq!(engine.transliterate("kh"), "х");
    assert_eq!(engine.transliterate("ts"), "ц");
    assert_eq!(engine.transliterate("ch"), "ч");
    assert_eq!(engine.transliterate("sh"), "ш");
    assert_eq!(engine.transliterate("shch"), "щ");
    assert_eq!(engine.transliterate("shchuka"), "щука");
    assert_eq!(engine.transliterate("zhizn"), "жизнь");
    assert_eq!(engine.transliterate("lyubov"), "любовь");
    assert_eq!(engine.transliterate("dobroe utro"), "доброе утро");
}

#[test]
fn test_mixed_case_and_punctuation() {
    let engine = Engine::new();
    assert_eq!(engine.transliterate("Dobroe Utro!"), "Доброе Утро!");
    assert_eq!(engine.transliterate("123 spasibo..."), "123 спасибо...");
    assert_eq!(
        engine.transliterate("kak dela? khorosho!"),
        "как дела? хорошо!"
    );
}

#[test]
fn test_suggestions() {
    let engine = Engine::new();
    let suggestions = engine.get_suggestions("priv");
    assert!(!suggestions.is_empty());
    assert!(suggestions
        .iter()
        .any(|s| s == "привет" || s.starts_with("прив")));

    let khorosho_sug = engine.get_suggestions("khorosho");
    assert!(khorosho_sug.iter().any(|s| s == "хорошо"));
}

#[test]
fn test_custom_mappings() {
    let mut engine = Engine::new();
    let mut rules = russian_keyboard_tool::transliteration::default_mappings();
    rules.push(russian_keyboard_tool::transliteration::MappingRule {
        pattern: "super".to_string(),
        replacement: "супер".to_string(),
    });
    engine.set_mappings(rules);
    assert_eq!(engine.transliterate("super"), "супер");
}

#[test]
fn test_all_specification_words() {
    let engine = Engine::new();
    let cases = vec![
        ("privet", "привет"),
        ("khorosho", "хорошо"),
        ("spasibo", "спасибо"),
        ("moskva", "москва"),
        ("ya", "я"),
        ("yu", "ю"),
        ("yo", "ё"),
        ("zh", "ж"),
        ("kh", "х"),
        ("ts", "ц"),
        ("ch", "ч"),
        ("sh", "ш"),
        ("shch", "щ"),
        ("zhizn", "жизнь"),
        ("lyubov", "любовь"),
        ("dobroe", "доброе"),
        ("utro", "утро"),
        ("den", "день"),
        ("vecher", "вечер"),
        ("shchuka", "щука"),
    ];

    for (input, expected) in cases {
        assert_eq!(
            engine.transliterate(input),
            expected,
            "Failed transliterating '{}': expected '{}', got '{}'",
            input,
            expected,
            engine.transliterate(input)
        );
    }
}
