//! The timeline search language: `field:value`, `-field:value`, `"quoted values"`, bare words for the message.
//! `AND` between terms is accepted and implied.

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
                "Unknown field '{name}'. Use host, user, event_id, level, source, process, verdict or pinned."
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

fn tokens(input: &str) -> Result<Vec<String>, QueryError> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut had_quote = false;
    for c in input.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                had_quote = true;
            }
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() || had_quote {
                    out.push(std::mem::take(&mut cur));
                }
                had_quote = false;
            }
            c => cur.push(c),
        }
    }
    if quoted {
        return Err(QueryError::UnclosedQuote);
    }
    if !cur.is_empty() || had_quote {
        out.push(cur);
    }
    Ok(out)
}

pub fn parse(input: &str) -> Result<Query, QueryError> {
    let mut terms = Vec::new();
    for tok in tokens(input)? {
        if tok == "AND" {
            continue;
        }
        if tok.chars().count() > MAX_VALUE {
            return Err(QueryError::TooLong);
        }
        let (negate, body) = match tok.strip_prefix('-') {
            Some(rest) if rest.contains(':') => (true, rest),
            _ => (false, tok.as_str()),
        };
        let term = match body.split_once(':') {
            Some((name, value)) if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') => {
                let field = FIELDS
                    .iter()
                    .find(|f| f.eq_ignore_ascii_case(name))
                    .copied()
                    .ok_or_else(|| QueryError::UnknownField(name.to_owned()))?;
                validate(field, value)?;
                Term::Field { field, value: value.to_owned(), negate }
            }
            _ => Term::Text(tok.clone()),
        };
        terms.push(term);
        if terms.len() > MAX_TERMS {
            return Err(QueryError::TooMany);
        }
    }
    Ok(Query { terms })
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

fn quote(v: &str) -> String {
    if v.is_empty() || v.chars().any(char::is_whitespace) { format!("\"{v}\"") } else { v.to_owned() }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Field { field, value, negate } => {
                write!(f, "{}{field}:{}", if *negate { "-" } else { "" }, quote(value))
            }
            Term::Text(t) => f.write_str(&quote(t)),
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
