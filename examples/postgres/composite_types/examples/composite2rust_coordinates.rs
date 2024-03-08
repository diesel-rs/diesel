// Function to connect to database.
use diesel_postgres_composite_type::establish_connection;

// Bring column names of the table into scope
use diesel_postgres_composite_type::schema::coordinates::{
    coord_id, dsl::coordinates, xcoord, ycoord,
};

// Define the signature of the SQL function we want to call:
use diesel::define_sql_function;
use diesel::sql_types::Integer;
define_sql_function!(fn distance_from_origin(re: Integer,im: Integer) -> Float);
define_sql_function!(fn shortest_distance() -> Record<(Integer,Float)>);
define_sql_function!(fn longest_distance() -> Record<(Integer,Float)>);

// Needed to select, construct the query and submit it.
use diesel::select;
use diesel::{QueryDsl, RunQueryDsl};

fn main() {
    let connection = &mut establish_connection();
    // Experiment 1: Read tuple directly from processed table
    let results: Vec<(i32, f32)> = coordinates
        .select((coord_id, distance_from_origin(xcoord, ycoord)))
        .load(connection)
        .expect("Error loading numbers");
    for r in results {
        println!("index {:?}, length {:?}", r.0, r.1);
    }
    // Experiment 2: Define a type for clearer re-use
    type Distance = (i32, f32);
    let results: Vec<Distance> = coordinates
        .select((coord_id, distance_from_origin(xcoord, ycoord)))
        .load(connection)
        .expect("Error loading numbers");
    for r in results {
        println!("index {:?}, length {:?}", r.0, r.1);
    }
    // Experiment 3: use tuple for single result and do some math in SQL
    // Notice that we only expect one result, not an vector
    // of results, so use get_result() instead of load())
    let result: Distance = select(shortest_distance())
        .get_result(connection)
        .expect("Error loading longest distance");
    println!(
        "Coordinate {:?} has shortest distance of {:?}",
        result.0, result.1
    );
    // Unfortunately, the members of our Distance struct, a tuple, are anonymous.
    // Will be unhandy for longer tuples.

    // Experiment 4: use composite type in SQL, read as Record in Rust
    // Notice that we only expect one result, not an vector
    // of results, so use get_result() instead of load())
    let result: Distance = select(longest_distance())
        .get_result(connection)
        .expect("Error loading longest distance");
    println!(
        "Coordinate {:?} has longest distance of {:?}",
        result.0, result.1
    );
    // TODO: also show an example with a recursively interpreted Record<Integer,Record<Integer,Integer>>
}
