pub fn is_positive_int(val: String) -> Result<(), String> {
    match val.parse::<u64>() {
        Ok(val) if val > 0 => Ok(()),
        // If the the value is <= 0 or can't be parsed
        _ => Err(format!("{} isn't a positive integer.", val)),
    }
}

#[cfg(test)]
mod tests {
    use super::is_positive_int;

    #[test]
    fn is_positive_int_should_parse_a_positive_integer_from_input_string() {
        assert_eq!(is_positive_int("1".to_string()), Ok(()))
    }

    #[test]
    fn is_positive_int_should_throw_an_error_with_zero() {
        assert_eq!(
            is_positive_int("0".to_string()),
            Err("0 isn't a positive integer.".to_string())
        )
    }

    #[test]
    fn is_positive_int_should_throw_an_error_with_negative_integer() {
        assert_eq!(
            is_positive_int("-5".to_string()),
            Err("-5 isn't a positive integer.".to_string())
        )
    }

    #[test]
    fn is_positive_int_should_throw_an_error_with_float() {
        assert_eq!(
            is_positive_int("5.2".to_string()),
            Err("5.2 isn't a positive integer.".to_string())
        )
    }
}
