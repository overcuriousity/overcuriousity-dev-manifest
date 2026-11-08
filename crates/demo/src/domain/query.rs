//! The timeline search language: `field:value`, `-field:value`, `"quoted values"`, bare words for the message.
//! `AND` between terms is accepted and implied. A token that starts with a quote is always text.

use std::fmt;

pub const FIELDS: [&str; 8] = ["host", "user", "event_id", "level", "source", "process", "verdict", "pinned"];
const MAX_TERMS: usize = 32;
const MAX_VALUE: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Field { field: &'static str, value: String, negate: bool },
    Text(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub terms: Vec<Term>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    UnknownField(String),
    UnclosedQuote,
    NotANumber(&'static str),
    TooLong,
    TooMany,
    BadValue(&'static str),
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownField(name) => write!(
                f,
                "Unknown field '{name}'. Use host, user, event_id, level, source, process, verdict or pinned. Put values that contain a colon in quotes."
            ),
            Self::UnclosedQuote => f.write_str("A quote is not closed."),
            Self::NotANumber(field) => write!(f, "{field} needs a number."),
            Self::TooLong => write!(f, "Keep each value under {MAX_VALUE} characters."),
            Self::TooMany => write!(f, "Use at most {MAX_TERMS} search terms."),
            Self::BadValue(field) => match *field {
                "verdict" => f.write_str("verdict is benign, suspicious, malicious or none."),
                _ => f.write_str("pinned is yes or no."),
            },
        }
    }
}

impl std::error::Error for QueryError {}

struct Token {
    text: String,
    quoted_start: bool,
}

/// Inside quotes, `\"` and `\\` are escapes; any other backslash is literal, so Windows paths need no doubling.
fn tokens(input: &str) -> Result<Vec<Token>, QueryError> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quoted, mut started, mut quoted_start) = (false, false, false);
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted_start |= !started;
                quoted = !quoted;
                started = true;
            }
            '\\' if quoted && matches!(chars.peek(), Some('"' | '\\')) => cur.extend(chars.next()),
            c if c.is_whitespace() && !quoted => {
                if started {
                    out.push(Token { text: std::mem::take(&mut cur), quoted_start });
                }
                (started, quoted_start) = (false, false);
            }
            c => {
                cur.push(c);
                started = true;
            }
        }
    }
    if quoted {
        return Err(QueryError::UnclosedQuote);
    }
    if started {
        out.push(Token { text: cur, quoted_start });
    }
    Ok(out)
}

pub fn parse(input: &str) -> Result<Query, QueryError> {
    let mut terms = Vec::new();
    for tok in tokens(input)? {
        if tok.text == "AND" && !tok.quoted_start {
            continue;
        }
        if tok.text.chars().count() > MAX_VALUE {
            return Err(QueryError::TooLong);
        }
        terms.push(if tok.quoted_start { Term::Text(tok.text) } else { field_term(tok.text)? });
        if terms.len() > MAX_TERMS {
            return Err(QueryError::TooMany);
        }
    }
    Ok(Query { terms })
}

fn field_term(tok: String) -> Result<Term, QueryError> {
    let (negate, body) = match tok.strip_prefix('-') {
        Some(rest) if rest.contains(':') => (true, rest),
        _ => (false, tok.as_str()),
    };
    match body.split_once(':') {
        Some((name, value)) if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') => {
            let field = FIELDS
                .iter()
                .find(|f| f.eq_ignore_ascii_case(name))
                .copied()
                .ok_or_else(|| QueryError::UnknownField(name.to_owned()))?;
            validate(field, value)?;
            Ok(Term::Field { field, value: value.to_owned(), negate })
        }
        _ => Ok(Term::Text(tok)),
    }
}

fn validate(field: &'static str, value: &str) -> Result<(), QueryError> {
    match field {
        "event_id" if value.parse::<i64>().is_err() => Err(QueryError::NotANumber(field)),
        "verdict" if !["benign", "suspicious", "malicious", "none"].contains(&value) => {
            Err(QueryError::BadValue(field))
        }
        "pinned" if !["yes", "no"].contains(&value) => Err(QueryError::BadValue(field)),
        _ => Ok(()),
    }
}

fn quoted(v: &str) -> String {
    format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Field { field, value, negate } => {
                let plain = !value.is_empty() && !value.contains(|c: char| c.is_whitespace() || c == '"');
                let v = if plain { value.clone() } else { quoted(value) };
                write!(f, "{}{field}:{v}", if *negate { "-" } else { "" })
            }
            Term::Text(t) => {
                let plain = !t.is_empty()
                    && !t.starts_with('-')
                    && !t.contains(|c: char| c.is_whitespace() || c == '"' || c == ':');
                f.write_str(&if plain && t != "AND" { t.clone() } else { quoted(t) })
            }
        }
    }
}

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.terms.iter().map(Term::to_string).collect();
        f.write_str(&parts.join(" "))
    }
}

impl Query {
    pub fn with(&self, term: Term) -> Query {
        let mut q = self.clone();
        q.terms.retain(|t| t != &term);
        q.terms.push(term);
        q
    }

    pub fn without(&self, index: usize) -> Query {
        let mut q = self.clone();
        if index < q.terms.len() {
            q.terms.remove(index);
        }
        q
    }
}
