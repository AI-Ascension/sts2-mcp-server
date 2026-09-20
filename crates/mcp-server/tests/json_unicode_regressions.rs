// SPDX-License-Identifier: MIT

use sts2_mcp_server::{JsonValue, parse_json};

#[test]
fn raw_unicode_scalar_boundaries_are_preserved() -> Result<(), String> {
    for text in [
        "ascii",
        "\u{80}\u{7ff}",
        "\u{800}\u{d7ff}\u{e000}\u{ffff}",
        "\u{10000}\u{10ffff}",
        "aé界🚀z",
    ] {
        let input = format!("\"{text}\"");
        assert_eq!(parse_json(&input)?, JsonValue::string(text));
    }
    Ok(())
}

#[test]
fn raw_and_escaped_unicode_match() -> Result<(), String> {
    let raw = r#""é界🚀""#;
    let escaped = r#""\u00e9\u754c\ud83d\ude80""#;
    assert_eq!(parse_json(raw)?, JsonValue::string("é界🚀"));
    assert_eq!(parse_json(raw)?, parse_json(escaped)?);
    Ok(())
}

#[test]
fn unicode_mixed_with_json_escapes_preserves_following_strings() -> Result<(), String> {
    let input = r#"["é\n界\"🚀\\","tail"]"#;
    let expected = JsonValue::Array(vec![
        JsonValue::string("é\n界\"🚀\\"),
        JsonValue::string("tail"),
    ]);
    assert_eq!(parse_json(input)?, expected);
    Ok(())
}

#[test]
fn unicode_keys_preserve_nested_structure() -> Result<(), String> {
    let input = r#"{"é":{"界":"🚀"},"tail":7}"#;
    let expected = JsonValue::object([
        (
            String::from("é"),
            JsonValue::object([(String::from("界"), JsonValue::string("🚀"))]),
        ),
        (String::from("tail"), JsonValue::Number(7)),
    ]);
    assert_eq!(parse_json(input)?, expected);
    Ok(())
}

#[test]
fn large_unicode_strings_preserve_following_tokens() -> Result<(), String> {
    // Exercise the formerly repeated suffix scan without a flaky timing threshold.
    for scalar in ["é", "界", "🚀"] {
        let text = scalar.repeat(16_000);
        let input = format!("[\"{text}\",17,true,null]");
        let expected = JsonValue::Array(vec![
            JsonValue::string(text),
            JsonValue::Number(17),
            JsonValue::Bool(true),
            JsonValue::Null,
        ]);
        assert_eq!(parse_json(&input)?, expected);
    }
    Ok(())
}

#[test]
fn duplicate_unicode_keys_are_refused() {
    for input in [r#"{"é":1,"\u00e9":2}"#, r#"{"🚀":1,"\ud83d\ude80":2}"#] {
        let result = parse_json(input);
        assert!(matches!(result, Err(error) if error.contains("duplicate JSON object key")));
    }
}

#[test]
fn malformed_suffixes_after_unicode_are_refused() {
    for input in [
        "\"é",
        "\"é\\q\"",
        "\"界\\u12\"",
        "\"🚀\\ud800\"",
        "\"🚀\\udc00\"",
        "\"é\n\"",
        "\"界\" false",
        r#"["🚀" true]"#,
    ] {
        assert!(parse_json(input).is_err(), "accepted malformed JSON");
    }
}

#[test]
fn diagnostics_keep_byte_offsets_after_unicode() {
    assert_eq!(
        parse_json("\"🚀\"x"),
        Err(String::from("unexpected trailing JSON input at byte 6")),
    );
    assert_eq!(
        parse_json("\"é\\q\""),
        Err(String::from("unsupported JSON escape at byte 5")),
    );
}
