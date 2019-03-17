fn capitalize(name: &str) -> String {
    name[..1].to_uppercase() + &name[1..]
}

pub fn to_camel_case(name: &str) -> String {
    name.split('_')
        .map(|word| capitalize(word))
        .collect::<String>()
}
