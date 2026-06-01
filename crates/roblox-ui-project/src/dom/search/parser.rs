/*!
    Recursive-descent parser for the Studio explorer search grammar.

    Precedence is `or` (lowest), then `and` (explicit or implicit via
    juxtaposition), then atoms. An atom is a parenthesised group, a property
    comparison (`Prop OP value`), a filter (`is:` / `tag:` / `prefix:`), a dotted
    ancestry path, or a bare name. A blank query parses to nothing.
*/

use super::ast::{Op, Query, Seg};
use super::lexer::{tokenize, Token};

/**
    Parse a search string into a [`Query`], or `None` if it is blank.
*/
pub(crate) fn parse(input: &str) -> Option<Query> {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return None;
    }
    let mut parser = Parser { tokens, pos: 0 };
    parser.parse_or()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.pos);
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn eat(&mut self, token: &Token) -> bool {
        if self.peek() == Some(token) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /**
        Consume and return a comparison operator, if the next token is one.
    */
    fn eat_op(&mut self) -> Option<Op> {
        match self.peek() {
            Some(&Token::Op(op)) => {
                self.pos += 1;
                Some(op)
            }
            _ => None,
        }
    }

    /**
        Whether the next token is the bare keyword `kw` (case-insensitive).
    */
    fn at_keyword(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(kw))
    }

    fn parse_or(&mut self) -> Option<Query> {
        let mut terms = vec![self.parse_and()?];
        while self.at_keyword("or") {
            self.pos += 1;
            match self.parse_and() {
                Some(term) => terms.push(term),
                None => break,
            }
        }
        Some(if terms.len() == 1 {
            terms.pop().unwrap()
        } else {
            Query::Or(terms)
        })
    }

    fn parse_and(&mut self) -> Option<Query> {
        let mut terms = Vec::new();
        loop {
            match self.peek() {
                None | Some(Token::RParen) => break,
                _ if self.at_keyword("or") => break,
                _ if self.at_keyword("and") => self.pos += 1,
                _ => match self.parse_atom() {
                    Some(atom) => terms.push(atom),
                    None => break,
                },
            }
        }
        match terms.len() {
            0 => None,
            1 => terms.pop(),
            _ => Some(Query::And(terms)),
        }
    }

    fn parse_atom(&mut self) -> Option<Query> {
        match self.peek()? {
            Token::LParen => {
                self.pos += 1;
                let inner = self.parse_or();
                self.eat(&Token::RParen);
                inner
            }
            Token::RParen | Token::Op(_) => None,
            Token::Word(_) | Token::Quoted(_) => self.parse_term(),
        }
    }

    /**
        Parse a single leading word/quoted term, optionally followed by a
        comparison operator and value (a property filter).
    */
    fn parse_term(&mut self) -> Option<Query> {
        let (text, quoted) = match self.advance()? {
            Token::Word(word) => (word.clone(), false),
            Token::Quoted(value) => (value.clone(), true),
            _ => return None,
        };

        if let Some(op) = self.eat_op() {
            let value = self.read_value()?;
            return Some(Query::Property {
                path: split_dots(&text),
                op,
                value,
            });
        }

        Some(if quoted {
            Query::Name(text)
        } else {
            self.classify(text)
        })
    }

    /**
        Read the value token following a comparison operator or `prefix:`.
    */
    fn read_value(&mut self) -> Option<String> {
        match self.advance()? {
            Token::Word(word) => Some(word.clone()),
            Token::Quoted(value) => Some(value.clone()),
            _ => None,
        }
    }

    /**
        Classify a bare word with no trailing operator into a leaf query.
    */
    fn classify(&mut self, word: String) -> Query {
        if let Some(rest) = word.strip_prefix("is:") {
            return Query::Is(self.value_after_prefix(rest));
        }
        if let Some(rest) = word.strip_prefix("tag:") {
            return Query::Tag(self.value_after_prefix(rest));
        }
        if let Some(idx) = word.find(':') {
            // `prefix:value` (e.g. `classname:decal`) is a partial property match.
            let value = self.value_after_prefix(&word[idx + 1..]);
            return Query::Property {
                path: split_dots(&word[..idx]),
                op: Op::Eq,
                value,
            };
        }
        if word.contains('.') {
            return Query::Ancestry(split_dots(&word).into_iter().map(seg).collect());
        }
        Query::Name(word)
    }

    /**
        The value of a `prefix:` filter: the inline `rest`, or the next token if
        the filter was written as `prefix: "quoted value"`.
    */
    fn value_after_prefix(&mut self, rest: &str) -> String {
        if rest.is_empty() {
            self.read_value().unwrap_or_default()
        } else {
            rest.to_string()
        }
    }
}

fn split_dots(s: &str) -> Vec<String> {
    s.split('.')
        .filter(|part| !part.is_empty())
        .map(String::from)
        .collect()
}

fn seg(s: String) -> Seg {
    match s.as_str() {
        "*" => Seg::AnyOne,
        "**" => Seg::AnyDepth,
        _ => Seg::Name(s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(input: &str) -> Query {
        parse(input).unwrap()
    }

    #[test]
    fn blank_is_none() {
        assert!(parse("   ").is_none());
    }

    #[test]
    fn names_and_filters() {
        assert_eq!(p("Wheel"), Query::Name("Wheel".into()));
        assert_eq!(p("is:Part"), Query::Is("Part".into()));
        assert_eq!(p("tag:Spin"), Query::Tag("Spin".into()));
        assert_eq!(
            p(r#"tag:"Light Source""#),
            Query::Tag("Light Source".into())
        );
    }

    #[test]
    fn property_comparisons() {
        assert_eq!(
            p("Anchored=true"),
            Query::Property {
                path: vec!["Anchored".into()],
                op: Op::Eq,
                value: "true".into()
            }
        );
        assert_eq!(
            p("Material == plas"),
            Query::Property {
                path: vec!["Material".into()],
                op: Op::Eq,
                value: "plas".into()
            }
        );
        assert_eq!(
            p("Health ~= 50"),
            Query::Property {
                path: vec!["Health".into()],
                op: Op::Ne,
                value: "50".into()
            }
        );
        assert_eq!(
            p("Position.X >= 1"),
            Query::Property {
                path: vec!["Position".into(), "X".into()],
                op: Op::Ge,
                value: "1".into()
            }
        );
        assert_eq!(
            p(r#"Size > "20, 5, 20""#),
            Query::Property {
                path: vec!["Size".into()],
                op: Op::Gt,
                value: "20, 5, 20".into()
            }
        );
    }

    #[test]
    fn ancestry_segments() {
        assert_eq!(
            p("Cart.*.Trim"),
            Query::Ancestry(vec![
                Seg::Name("Cart".into()),
                Seg::AnyOne,
                Seg::Name("Trim".into())
            ])
        );
        assert_eq!(
            p("Parent.**"),
            Query::Ancestry(vec![Seg::Name("Parent".into()), Seg::AnyDepth])
        );
    }

    #[test]
    fn boolean_precedence_and_grouping() {
        // `a b or c` is `(a AND b) OR c`.
        match p("Cat Dog or Bird") {
            Query::Or(parts) => {
                assert!(matches!(parts[0], Query::And(_)));
                assert_eq!(parts[1], Query::Name("Bird".into()));
            }
            other => panic!("expected Or, got {other:?}"),
        }
        assert!(matches!(p("(Cat or Dog)"), Query::Or(_)));
        assert!(matches!(p("Anchored=true is:Part"), Query::And(_)));
    }
}
