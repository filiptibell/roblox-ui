/*!
    Lexer for the Studio explorer search grammar.

    Whitespace separates tokens but is otherwise insignificant. Comparison
    operators may be written with or without surrounding spaces (`Anchored=true`
    and `Anchored = true` lex identically); the two-character operators (`==`,
    `~=`, `>=`, `<=`) win over their single-character prefixes via maximal munch.
    A `"`-quoted run becomes a single [`Token::Quoted`], letting names and values
    contain spaces.
*/

use super::ast::Op;

/**
    A single lexical token of a search string.
*/
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    LParen,
    RParen,
    Op(Op),
    Word(String),
    Quoted(String),
}

/**
    Whether `c` may appear in a bare word (a name, filter, ancestry path, or
    property/value). Everything that is not whitespace, a parenthesis, a quote,
    or part of an operator is a word character - including `.`, `:`, `*`, `_`
    and `-`.
*/
fn is_word_char(c: char) -> bool {
    !c.is_whitespace() && !matches!(c, '(' | ')' | '=' | '~' | '>' | '<' | '"')
}

pub(crate) fn tokenize(input: &str) -> Vec<Token> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match c {
            c if c.is_whitespace() => i += 1,
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            // Comparison operators. `=` and `==` are both equality; `~` is only
            // meaningful as part of `~=` and is otherwise ignored.
            '=' => {
                tokens.push(Token::Op(Op::Eq));
                i += if next == Some('=') { 2 } else { 1 };
            }
            '~' if next == Some('=') => {
                tokens.push(Token::Op(Op::Ne));
                i += 2;
            }
            '~' => i += 1,
            '>' => {
                tokens.push(Token::Op(if next == Some('=') { Op::Ge } else { Op::Gt }));
                i += if next == Some('=') { 2 } else { 1 };
            }
            '<' => {
                tokens.push(Token::Op(if next == Some('=') { Op::Le } else { Op::Lt }));
                i += if next == Some('=') { 2 } else { 1 };
            }
            '"' => {
                let mut value = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    value.push(chars[i]);
                    i += 1;
                }
                i += 1; // closing quote (or end of input)
                tokens.push(Token::Quoted(value));
            }
            _ => {
                let start = i;
                while i < chars.len() && is_word_char(chars[i]) {
                    i += 1;
                }
                tokens.push(Token::Word(chars[start..i].iter().collect()));
            }
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_and_operators() {
        assert_eq!(
            tokenize("Anchored=true"),
            vec![
                Token::Word("Anchored".into()),
                Token::Op(Op::Eq),
                Token::Word("true".into())
            ]
        );
        // Operators may be spaced, and the two-char forms win.
        assert_eq!(
            tokenize("Health >= 50"),
            vec![
                Token::Word("Health".into()),
                Token::Op(Op::Ge),
                Token::Word("50".into())
            ]
        );
        assert_eq!(tokenize("x ~= y")[1], Token::Op(Op::Ne));
    }

    #[test]
    fn quoted_runs_keep_spaces() {
        assert_eq!(
            tokenize(r#"tag: "Light Source""#),
            vec![
                Token::Word("tag:".into()),
                Token::Quoted("Light Source".into())
            ]
        );
    }

    #[test]
    fn dotted_and_filter_words_are_single_tokens() {
        assert_eq!(
            tokenize("Cart.*.Trim"),
            vec![Token::Word("Cart.*.Trim".into())]
        );
        assert_eq!(tokenize("is:Part"), vec![Token::Word("is:Part".into())]);
    }
}
