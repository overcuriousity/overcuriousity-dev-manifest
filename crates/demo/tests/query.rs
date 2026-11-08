#![allow(clippy::unwrap_used)]

use demo::domain::query::{Term, parse};

#[test]
fn field_terms_quoted_values_and_negation_parse() {
    let q = parse(r#"host:ws-01 AND -user:"svc backup" event_id:4624"#).unwrap();
    assert_eq!(
        q.terms,
        vec![
            Term::Field {
                field: "host",
                value: "ws-01".into(),
                negate: false
            },
            Term::Field {
                field: "user",
                value: "svc backup".into(),
                negate: true
            },
            Term::Field {
                field: "event_id",
                value: "4624".into(),
                negate: false
            },
        ]
    );
    assert_eq!(
        q.to_string(),
        r#"host:ws-01 -user:"svc backup" event_id:4624"#
    );
}

#[test]
fn bare_words_match_message_and_empty_input_matches_everything() {
    assert_eq!(
        parse("mimikatz").unwrap().terms,
        vec![Term::Text("mimikatz".into())]
    );
    assert!(parse("   ").unwrap().terms.is_empty());
}

#[test]
fn errors_are_one_sentence() {
    assert_eq!(
        parse("foo:bar").unwrap_err().to_string(),
        "Unknown field 'foo'. Use host, user, event_id, level, source, process, verdict or pinned."
    );
    assert_eq!(
        parse(r#"user:"x"#).unwrap_err().to_string(),
        "A quote is not closed."
    );
    assert_eq!(
        parse("event_id:abc").unwrap_err().to_string(),
        "event_id needs a number."
    );
    let many = (0..33)
        .map(|i| format!("w{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        parse(&many).unwrap_err().to_string(),
        "Use at most 32 search terms."
    );
}
