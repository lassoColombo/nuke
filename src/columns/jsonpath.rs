//! The JSONPath dialect a CRD's `additionalPrinterColumns` are written in:
//! `k8s.io/client-go/util/jsonpath`, in the "simple path" form the
//! apiserver accepts for a printer column (one expression, no template).
//!
//! ```text
//! .spec.replicas                                  a member
//! .metadata.labels.app\.kubernetes\.io/name       a member with escaped dots
//! .spec.ports[*].port                             every element
//! .status.addresses[0].ip      [-1]  [0:2]        an index, from the end, a range
//! .status.conditions[?(@.type=="Ready")].status   a filter (== != < <= > >=)
//! ..name                                          every descendant's member
//! ```
//!
//! [`eval`] returns every node the path reaches, in document order, as the
//! apiserver's `FindResults` does; a missing member reaches nothing rather
//! than failing, as its `allowMissingKeys` does. What a column makes of
//! several results is the column's business ([`super::Column`]).

use serde_json::Value as Json;

/// A parsed path.
#[derive(Debug, Clone, PartialEq)]
pub struct Path(Vec<Segment>);

#[derive(Debug, Clone, PartialEq)]
enum Segment {
    /// `.name` or `['name']`.
    Member(String),
    /// `.*` or `[*]`.
    Wildcard,
    /// `..name`.
    Recursive(String),
    /// `[n]`, negative from the end.
    Index(i64),
    /// `[start:end]`, Python-style, end exclusive.
    Range(Option<i64>, Option<i64>),
    /// `[?(@.path op literal)]`, or `[?(@.path)]` for existence.
    Filter(Path, Option<(Op, Literal)>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq)]
enum Literal {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
}

impl Path {
    /// Parse `text`, which may start with `$` or `{` as kubectl allows.
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        let text = text
            .strip_prefix('{')
            .map_or(text, |t| t.strip_suffix('}').unwrap_or(t));
        let text = text.strip_prefix('$').unwrap_or(text);
        let mut p = Parser {
            s: text.as_bytes(),
            i: 0,
        };
        let path = p.path(false)?;
        if p.i != p.s.len() {
            return Err(format!("unexpected `{}` at {}", p.s[p.i] as char, p.i));
        }
        Ok(path)
    }

    /// Every node `root` reaches through the path, in document order.
    pub fn eval<'a>(&self, root: &'a Json) -> Vec<&'a Json> {
        let mut current = vec![root];
        for segment in &self.0 {
            let mut next = Vec::new();
            for node in current {
                segment.apply(node, &mut next);
            }
            current = next;
        }
        current
    }
}

impl Segment {
    fn apply<'a>(&self, node: &'a Json, out: &mut Vec<&'a Json>) {
        match self {
            Segment::Member(name) => {
                if let Some(v) = node.get(name) {
                    out.push(v);
                }
            }
            Segment::Wildcard => match node {
                Json::Array(items) => out.extend(items),
                Json::Object(map) => out.extend(map.values()),
                _ => {}
            },
            Segment::Recursive(name) => {
                let mut nodes = Vec::new();
                descendants(node, &mut nodes);
                for n in nodes {
                    if let Some(v) = n.get(name) {
                        out.push(v);
                    }
                }
            }
            Segment::Index(i) => {
                if let Some(items) = node.as_array()
                    && let Some(v) = at(items, *i)
                {
                    out.push(v);
                }
            }
            Segment::Range(start, end) => {
                if let Some(items) = node.as_array() {
                    let len = items.len() as i64;
                    let clamp = |i: i64| i.clamp(0, len) as usize;
                    let s = clamp(start.map_or(0, |s| if s < 0 { s + len } else { s }));
                    let e = clamp(end.map_or(len, |e| if e < 0 { e + len } else { e }));
                    if s < e {
                        out.extend(&items[s..e]);
                    }
                }
            }
            Segment::Filter(path, test) => {
                if let Some(items) = node.as_array() {
                    out.extend(items.iter().filter(|item| holds(path, test, item)));
                }
            }
        }
    }
}

/// `node` and every node under it, depth first.
fn descendants<'a>(node: &'a Json, out: &mut Vec<&'a Json>) {
    out.push(node);
    match node {
        Json::Array(items) => items.iter().for_each(|v| descendants(v, out)),
        Json::Object(map) => map.values().for_each(|v| descendants(v, out)),
        _ => {}
    }
}

fn at(items: &[Json], i: i64) -> Option<&Json> {
    let i = if i < 0 { i + items.len() as i64 } else { i };
    usize::try_from(i).ok().and_then(|i| items.get(i))
}

/// Whether `item` passes the filter: its `path` reaches something, and
/// that something compares as asked. A comparison across types is false.
fn holds(path: &Path, test: &Option<(Op, Literal)>, item: &Json) -> bool {
    let Some(value) = path.eval(item).into_iter().next() else {
        return false;
    };
    let Some((op, literal)) = test else {
        return !value.is_null();
    };
    let ordering = match (value, literal) {
        (Json::String(a), Literal::Str(b)) => a.as_str().cmp(b.as_str()),
        (Json::Number(a), Literal::Num(b)) => match a.as_f64().and_then(|a| a.partial_cmp(b)) {
            Some(o) => o,
            None => return false,
        },
        (Json::Bool(a), Literal::Bool(b)) => a.cmp(b),
        (Json::Null, Literal::Null) => std::cmp::Ordering::Equal,
        _ => return *op == Op::Ne,
    };
    use std::cmp::Ordering::*;
    match op {
        Op::Eq => ordering == Equal,
        Op::Ne => ordering != Equal,
        Op::Lt => ordering == Less,
        Op::Le => ordering != Greater,
        Op::Gt => ordering == Greater,
        Op::Ge => ordering != Less,
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn eat(&mut self, c: u8) -> bool {
        match self.peek() == Some(c) {
            true => {
                self.i += 1;
                true
            }
            false => false,
        }
    }

    fn skip_spaces(&mut self) {
        while self.peek() == Some(b' ') {
            self.i += 1;
        }
    }

    /// Segments until the end, or, inside a filter, until an operator or
    /// the closing parenthesis.
    fn path(&mut self, in_filter: bool) -> Result<Path, String> {
        let mut segments = Vec::new();
        loop {
            match self.peek() {
                Some(b'.') => {
                    self.i += 1;
                    match self.eat(b'.') {
                        true => {
                            let name = self.name()?;
                            segments.push(Segment::Recursive(name));
                        }
                        false if self.eat(b'*') => segments.push(Segment::Wildcard),
                        false => segments.push(Segment::Member(self.name()?)),
                    }
                }
                Some(b'[') => {
                    self.i += 1;
                    segments.push(self.subscript()?);
                    if !self.eat(b']') {
                        return Err(format!("expected `]` at {}", self.i));
                    }
                }
                Some(b' ' | b')' | b'=' | b'!' | b'<' | b'>') if in_filter => break,
                None => break,
                Some(c) => return Err(format!("unexpected `{}` at {}", c as char, self.i)),
            }
        }
        Ok(Path(segments))
    }

    /// A member name after a dot: up to the next `.` or `[`, a `\.`
    /// standing for a literal dot.
    fn name(&mut self) -> Result<String, String> {
        // Bytes, decoded once: a name may hold any UTF-8, and every
        // delimiter is ASCII, so none splits a character.
        let mut name = Vec::new();
        while let Some(c) = self.peek() {
            match c {
                b'\\' if self.s.get(self.i + 1) == Some(&b'.') => {
                    name.push(b'.');
                    self.i += 2;
                }
                b'.' | b'[' | b' ' | b')' | b'=' | b'!' | b'<' | b'>' => break,
                _ => {
                    name.push(c);
                    self.i += 1;
                }
            }
        }
        let name = String::from_utf8(name).map_err(|e| e.to_string())?;
        match name.is_empty() {
            true => Err(format!("expected a name at {}", self.i)),
            false => Ok(name),
        }
    }

    fn subscript(&mut self) -> Result<Segment, String> {
        self.skip_spaces();
        match self.peek() {
            Some(b'*') => {
                self.i += 1;
                Ok(Segment::Wildcard)
            }
            Some(b'\'' | b'"') => Ok(Segment::Member(self.quoted()?)),
            Some(b'?') => {
                self.i += 1;
                if !self.eat(b'(') {
                    return Err(format!("expected `(` at {}", self.i));
                }
                self.skip_spaces();
                if !self.eat(b'@') {
                    return Err(format!("expected `@` at {}", self.i));
                }
                let path = self.path(true)?;
                self.skip_spaces();
                let test = match self.peek() {
                    Some(b')') => None,
                    _ => {
                        let op = self.op()?;
                        self.skip_spaces();
                        let literal = self.literal()?;
                        Some((op, literal))
                    }
                };
                self.skip_spaces();
                if !self.eat(b')') {
                    return Err(format!("expected `)` at {}", self.i));
                }
                Ok(Segment::Filter(path, test))
            }
            _ => {
                let start = self.int()?;
                self.skip_spaces();
                match self.eat(b':') {
                    true => {
                        self.skip_spaces();
                        let end = self.int()?;
                        Ok(Segment::Range(start, end))
                    }
                    false => match start {
                        Some(i) => Ok(Segment::Index(i)),
                        None => Err(format!("expected an index at {}", self.i)),
                    },
                }
            }
        }
    }

    fn quoted(&mut self) -> Result<String, String> {
        let quote = self.peek().unwrap();
        self.i += 1;
        let start = self.i;
        while let Some(c) = self.peek() {
            if c == quote {
                let text =
                    std::str::from_utf8(&self.s[start..self.i]).map_err(|e| e.to_string())?;
                self.i += 1;
                return Ok(text.to_string());
            }
            self.i += 1;
        }
        Err("unterminated string".into())
    }

    fn int(&mut self) -> Result<Option<i64>, String> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.i += 1;
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).unwrap();
        match text {
            "" => Ok(None),
            _ => text
                .parse()
                .map(Some)
                .map_err(|e| format!("{e} at {start}")),
        }
    }

    fn op(&mut self) -> Result<Op, String> {
        let two = |p: &mut Self, a: u8, b: u8| p.peek() == Some(a) && p.s.get(p.i + 1) == Some(&b);
        let (op, len) = match self.peek() {
            Some(b'=') if two(self, b'=', b'=') => (Op::Eq, 2),
            Some(b'!') if two(self, b'!', b'=') => (Op::Ne, 2),
            Some(b'<') if two(self, b'<', b'=') => (Op::Le, 2),
            Some(b'>') if two(self, b'>', b'=') => (Op::Ge, 2),
            Some(b'<') => (Op::Lt, 1),
            Some(b'>') => (Op::Gt, 1),
            _ => return Err(format!("expected an operator at {}", self.i)),
        };
        self.i += len;
        Ok(op)
    }

    fn literal(&mut self) -> Result<Literal, String> {
        match self.peek() {
            Some(b'\'' | b'"') => Ok(Literal::Str(self.quoted()?)),
            _ => {
                let start = self.i;
                while matches!(self.peek(), Some(c) if !matches!(c, b' ' | b')')) {
                    self.i += 1;
                }
                let text = std::str::from_utf8(&self.s[start..self.i]).unwrap();
                match text {
                    "true" => Ok(Literal::Bool(true)),
                    "false" => Ok(Literal::Bool(false)),
                    "null" => Ok(Literal::Null),
                    _ => text
                        .parse()
                        .map(Literal::Num)
                        .map_err(|_| format!("bad literal `{text}` at {start}")),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn probe() -> Json {
        json!({
            "metadata": {"name": "p1", "labels": {"a.b/c": "labelled", "città": "milano"}},
            "spec": {
                "arr": ["x", "y"],
                "num": 42,
                "items": [{"name": "one", "kind": "a", "n": 1}, {"name": "two", "kind": "b", "n": 2}, {"name": "three", "kind": "b", "n": 3}],
                "conditions": [{"type": "Ready", "status": "True"}, {"type": "Valid", "status": "False"}],
                "nul": null
            }
        })
    }

    fn strings(path: &str) -> Vec<String> {
        Path::parse(path)
            .unwrap()
            .eval(&probe())
            .into_iter()
            .map(|v| match v {
                Json::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect()
    }

    #[test]
    fn members_and_escapes() {
        assert_eq!(strings(".spec.num"), ["42"]);
        assert_eq!(strings(".spec.missing"), Vec::<String>::new());
        assert_eq!(strings(".spec.nul"), ["null"]);
        assert_eq!(strings(".metadata.labels.a\\.b/c"), ["labelled"]);
        assert_eq!(strings(".metadata.labels['a.b/c']"), ["labelled"]);
        assert_eq!(strings(".metadata.labels.città"), ["milano"]);
        assert_eq!(strings("$.metadata.name"), ["p1"]);
        assert_eq!(strings("{.metadata.name}"), ["p1"]);
        // A member of an array reaches nothing, as the apiserver's does.
        assert_eq!(strings(".spec.items.name"), Vec::<String>::new());
    }

    #[test]
    fn arrays() {
        assert_eq!(strings(".spec.arr[*]"), ["x", "y"]);
        assert_eq!(strings(".spec.items[*].name"), ["one", "two", "three"]);
        assert_eq!(strings(".spec.items[1].name"), ["two"]);
        assert_eq!(strings(".spec.items[-1].name"), ["three"]);
        assert_eq!(strings(".spec.items[0:2].name"), ["one", "two"]);
        assert_eq!(strings(".spec.items[1:].name"), ["two", "three"]);
        assert_eq!(strings(".spec.items[:-1].name"), ["one", "two"]);
        assert_eq!(strings(".spec.items[9].name"), Vec::<String>::new());
        assert_eq!(strings(".spec.num[0]"), Vec::<String>::new());
    }

    #[test]
    fn filters() {
        assert_eq!(
            strings(".spec.conditions[?(@.type==\"Ready\")].status"),
            ["True"]
        );
        assert_eq!(
            strings(".spec.conditions[?(@.type == 'Valid')].status"),
            ["False"]
        );
        assert_eq!(
            strings(".spec.items[?(@.kind=='b')].name"),
            ["two", "three"]
        );
        assert_eq!(strings(".spec.items[?(@.kind!='b')].name"), ["one"]);
        assert_eq!(strings(".spec.items[?(@.n>1)].name"), ["two", "three"]);
        assert_eq!(strings(".spec.items[?(@.n<=1)].name"), ["one"]);
        assert_eq!(strings(".spec.items[?(@.n)].name"), ["one", "two", "three"]);
        assert_eq!(
            strings(".spec.items[?(@.absent)].name"),
            Vec::<String>::new()
        );
        assert_eq!(
            strings(".spec.items[?(@.kind==1)].name"),
            Vec::<String>::new()
        );
        assert_eq!(
            strings(".spec.items[?(@.kind!=1)].name"),
            ["one", "two", "three"]
        );
    }

    #[test]
    fn recursive() {
        assert_eq!(strings("..name")[0], "p1");
        assert_eq!(strings("..name").len(), 4);
    }

    #[test]
    fn rejects_what_it_cannot_read() {
        assert!(Path::parse(".spec.items[0,1]").is_err());
        assert!(Path::parse(".spec.items[?(@.a =~ 'x')]").is_err());
        assert!(Path::parse(".spec.").is_err());
        assert!(Path::parse("spec").is_err());
        assert!(Path::parse(".spec[").is_err());
    }
}
