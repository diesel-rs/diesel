use std::env;

fn main() {
    if env::var("CARGO_PKG_VERSION").unwrap() == "1.3.0" {
        panic!(
            "Did you remember to publish documentation on the config file? \
             If not go do it. And then delete this build script."
        );
    }
}
