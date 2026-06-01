/*!
    Minimal JSONC support: strip `//` line and `/* */` block comments so that
    `serde_json` can parse `.jsonc` / commented project & model files. String
    contents (and escapes) are preserved verbatim. Trailing commas are not
    handled (comments are by far the common case in Rojo projects).
*/

pub(crate) fn strip(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escape = false;

    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '/' => match chars.peek() {
                Some('/') => {
                    chars.next();
                    for n in chars.by_ref() {
                        if n == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                }
                Some('*') => {
                    chars.next();
                    let mut prev = '\0';
                    for n in chars.by_ref() {
                        if prev == '*' && n == '/' {
                            break;
                        }
                        prev = n;
                    }
                }
                _ => out.push(c),
            },
            _ => out.push(c),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::strip;

    #[test]
    fn strips_line_and_block_comments() {
        let input = r#"{
            // a line comment
            "a": 1, /* inline */ "b": "keep // not a comment",
            /* block
               comment */
            "c": true
        }"#;
        let stripped = strip(input);
        let value: serde_json::Value = serde_json::from_str(&stripped).unwrap();
        assert_eq!(value["a"], 1);
        assert_eq!(value["b"], "keep // not a comment");
        assert_eq!(value["c"], true);
    }
}
