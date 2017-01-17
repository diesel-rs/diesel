use std::io::{Write, Result};

/// Simple pretty printer hand tailored for the output generated
/// by the `quote` - crate for schema inference
///
/// Rules:
/// 
///1. Seeing `{` increases indentation level
///2. Seeing `}` decreases indentation level
///3. Insert newline after `{`, `}`, `,`, and `;`
///4. Don't put spaces:
///  - between ident and `!`,
///  - between path segments and `::`
///  - after `(`, '<' and before `)`, `>`
///  - before `,`
pub fn format_schema<W: Write>(schema: &str, mut output: W) -> Result<()> {
    let mut out = String::new();
    let mut indent = String::new();
    let mut skip_space = false;
    let mut last_char = ' ';
    
    for c in schema.chars() {
        // quote inserts whitespaces at some strange location,
        // remove them
        match c {
            '!' | '(' | ';' | ',' | '<' | ')' | '>' if last_char.is_whitespace()
                => { out.pop(); }
            ':' if last_char.is_whitespace() => {
                // check if we are not at the beginning of a
                // fully qualified path and than remove the whitespace
                let char_before_whitespace = {
                    let mut chars = out.chars();
                    chars.next_back();
                    chars.next_back()
                };
                if char_before_whitespace != Some('>') {
                    out.pop();
                }
            }
            _=> {}
        }
        if skip_space && c.is_whitespace() && last_char != '>' {
            skip_space = false;
            continue;
        }
        last_char = c;
        skip_space = false;
        
        // there is already an empty line before {
        // we need to remove the already inserted indent, because the
        // new indent is smaller than the old one
        if c == '}' {
            while let Some(c) = out.pop() {
                if c == '\n' {
                    break;
                }
            }
            indent.pop();
            out = format!("{}\n{}", out, indent);
        }
        // push the current token to the output string
        out = format!("{}{}", out, c);
        
        // we need to insert newlines in some places and adjust the indent
        // also we need to remember if we could skip the next whitespace
        match c {
            ';' | ',' | '}'  => {
                skip_space = true;
                out = format!("{}\n{}", out, indent);
            }
            ':' | '(' => skip_space = true,
            '{' => {
                skip_space = true;
                indent += "\t";
                out = format!("{}\n{}", out, indent);
            }
            _=>{}
        }        
    }
    write!(&mut output, "{}", out.replace("\t","    "))
}


#[cfg(test)]
mod tests {
    use super::format_schema;
    use std::io::Cursor;

    fn run_test(input: &str, expected: &str){
        let out = Vec::<u8>::new();
        let mut c = Cursor::new(out);
        format_schema(input, &mut c).unwrap();
        let actual = String::from_utf8(c.into_inner()).unwrap();
        assert_eq!(expected, actual);
    }
    
    #[test]
    fn test_remove_whitespace_colon() {
        run_test(":: diesel :: types :: Text", "::diesel::types::Text");
    }

    #[test]
    fn test_format_nullable() {
        run_test("Nullable < :: diesel :: types :: Text >",
            "Nullable<::diesel::types::Text>");
    }

    #[test]
    fn test_newline_after_comma() {
        run_test(",", ",\n");
    }

    #[test]
    fn test_increase_indent() {
        run_test("{","{\n    ");
    }

    #[test]
    fn test_decrease_indent() {
        run_test("{abc,}","{\n    abc,\n}\n");
    }
    
    #[test]
    fn test_format_arrow() {
        run_test("created_at -> :: diesel :: types :: Timestamp",
            "created_at -> ::diesel::types::Timestamp");
    }

    #[test]
    fn test_format_full_line() {
        run_test("created_at -> :: diesel :: types :: Timestamp ,",
            "created_at -> ::diesel::types::Timestamp,\n");
    }

    #[test]
    fn test_format_include_line() {
        run_test("pub use self :: infer_locks :: * ;",
            "pub use self::infer_locks::*;\n");
    }

    #[test]
    fn test_format_generated_mod() {
        run_test("mod infer_users { table ! { users ( id ) { id -> :: diesel :: types :: Int4 , username -> :: diesel :: types :: Varchar , password -> :: diesel :: types :: Varchar , } } } pub use self :: infer_users :: * ;",
r"mod infer_users {
    table! {
        users(id) {
            id -> ::diesel::types::Int4,
            username -> ::diesel::types::Varchar,
            password -> ::diesel::types::Varchar,
        }
    }
}
pub use self::infer_users::*;
");
    }
    
}
