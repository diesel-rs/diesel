use diesel::sqlite::SqliteValue;

fn _test(mut value: SqliteValue<'_, '_, '_>) {
    let first = value.read_blob();
    let second = value.read_text();
    //~^ ERROR: cannot borrow `value` as mutable more than once at a time
    let third = value.read_blob();
    //~^ ERROR: cannot borrow `value` as mutable more than once at a time
    let _ = value.read_integer();
    //~^ ERROR: cannot borrow `value` as mutable more than once at a time
    let _ = value.read_long();
    //~^ ERROR: cannot borrow `value` as mutable more than once at a time
    let _ = value.read_double();
    //~^ ERROR: cannot borrow `value` as mutable more than once at a time
    // need to print the data here
    // to trigger the error
    println!("{first:?}, {second}, {third:?}");
}

// these cases should compile
fn _test2(mut value: SqliteValue<'_, '_, '_>) {
    let _int = value.read_integer();
    {
        let _first = value.read_blob();
    }
    {
        let _second = value.read_text();
    }
}

fn main() {}
