

/// Simple pretty printer hand tailored for the output generated
/// by the `quote` - crate for schema inference
///
/// Rules:
///   1. Seeing { increases indentation level
///   2. Seeing } decreases indentation level
///   3. Insert newline after {, }, ,, and ;
///   4. Don't put spaces:
///      between ident and !,
///      between path segments and ::
///      after ( and before )
///      before ,
pub fn format_schema<W>(schema: &str, mut output: W) where W: ::std::io::Write {
    let mut out = String::new();
    let mut indent = String::new();
    let mut skip_space = false;
    let mut last_char = ' ';
    
    for c in schema.chars() {
        // quote inserts whitespaces at some strange location,
        // remove them:        
        match c {
            '!' | '(' | ';' | ',' | '<' | ')' | '>' if last_char.is_whitespace()
                => { out.pop(); }
            ':' if last_char.is_whitespace() => {
                out.pop();
                match out.pop() {
                    Some(c @ '>') => {
                        out.push(c);
                        out += " ";
                    }
                    Some(c) => {
                        out.push(c);
                    }
                    _ => {}
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
            out += "\n";
            out += &indent;
        }
        // push the current token to the output string
        out = format!("{}{}", out, c);
        
        // we need to insert newlines in some places and adjust the indent
        // also we need to remember if we could skip the next whitespace
        match c {
            ';' | ',' | '}'  => {
                skip_space = true;
                out += "\n";
                out += &indent;
            }
            ':' | '(' => skip_space = true,
            '{' => {
                skip_space = true;
                out += "\n";
                indent += "\t";
                out += &indent;
            }
            _=>{}
        }        
    }
    write!(&mut output, "{}", out).unwrap();
}


#[cfg(test)]
mod tests {
    use super::format_schema;
    use std::io::Cursor;
    
    #[test]
    fn test_remove_whitespace_colon() {
        let out = Vec::<u8>::new();
        let mut c = Cursor::new(out);
        let input = ":: diesel :: types :: Text";
        format_schema(input, &mut c);
        assert_eq!("::diesel::types::Text",
                   String::from_utf8(c.into_inner()).unwrap());
    }

    #[test]
    fn test_format_nullable() {
        let out = Vec::<u8>::new();
        let mut c = Cursor::new(out);
        let input = "Nullable < :: diesel :: types :: Text >";
        format_schema(input, &mut c);
        assert_eq!("Nullable<::diesel::types::Text>",
                   String::from_utf8(c.into_inner()).unwrap());
    }

    #[test]
    fn test_newline_after_comma() {
        let out = Vec::<u8>::new();
        let mut c = Cursor::new(out);
        let input = ",";
        format_schema(input, &mut c);
        assert_eq!(",\n",
                   String::from_utf8(c.into_inner()).unwrap());
    }

    #[test]
    fn test_increase_indent() {
        let out = Vec::<u8>::new();
        let mut c = Cursor::new(out);
        let input = "{";
        format_schema(input, &mut c);
        assert_eq!("{\n\t",
                   String::from_utf8(c.into_inner()).unwrap());
    }

    #[test]
    fn test_decrease_indent() {
        let out = Vec::<u8>::new();
        let mut c = Cursor::new(out);
        let input = "{abc,}";
        format_schema(input, &mut c);
        assert_eq!("{\n\tabc,\n}\n",
                   String::from_utf8(c.into_inner()).unwrap());
    }
    
    #[test]
    fn test_format_arrow() {
        let out = Vec::<u8>::new();
        let mut c = Cursor::new(out);
        let input = "created_at -> :: diesel :: types :: Timestamp";
        format_schema(input, &mut c);
        assert_eq!("created_at -> ::diesel::types::Timestamp",
                   String::from_utf8(c.into_inner()).unwrap());
    }

    #[test]
    fn test_format_full_line() {
        let out = Vec::<u8>::new();
        let mut c = Cursor::new(out);
        let input = "created_at -> :: diesel :: types :: Timestamp ,";
        format_schema(input, &mut c);
        assert_eq!("created_at -> ::diesel::types::Timestamp,\n",
                   String::from_utf8(c.into_inner()).unwrap());
    }

    #[test]
    fn test_format_include_line() {
        let out = Vec::<u8>::new();
        let mut c = Cursor::new(out);
        let input = "pub use self :: infer_locks :: * ;";
        format_schema(input, &mut c);
        assert_eq!("pub use self::infer_locks::*;\n",
                   String::from_utf8(c.into_inner()).unwrap());
    }

    #[test]
    fn test_format_generated_mod() {
        let out = Vec::<u8>::new();
        let mut c = Cursor::new(out);
        let input = "mod infer_users { table ! { users ( id ) { id -> :: diesel :: types :: Int4 , username -> :: diesel :: types :: Varchar , password -> :: diesel :: types :: Varchar , } } } pub use self :: infer_users :: * ;";
        format_schema(input, &mut c);
        let out = String::from_utf8(c.into_inner()).unwrap();
        let expect =
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
";
        assert_eq!(expect,out);
    }
    
}
