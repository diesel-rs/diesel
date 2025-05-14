// Function to connect to database.
use composite_types::establish_connection;

// Bring column names of the table into scope
use composite_types::schema::colors::{blue, color_id, color_name, dsl::colors, green, red};

// Define the signature of the SQL function we want to call:
use diesel::declare_sql_function;
use diesel::pg::Pg;
use diesel::pg::PgValue;
use diesel::sql_types::{Float, Integer, Record, Text};

#[declare_sql_function]
extern "SQL" {
    fn color2grey(r: Integer, g: Integer, b: Integer) -> Record<(Float, Text)>;
    fn color2gray(r: Integer, g: Integer, b: Integer) -> PgGrayType;
}

// Needed to select, construct the query and submit it.
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::{QueryDsl, RunQueryDsl};

#[derive(Debug, FromSqlRow)]
pub struct GrayType {
    pub intensity: f32,
    pub suggestion: String,
}

// Define how a record of this can be converted to a Postgres type.
type PgGrayType = Record<(Float, Text)>;

// Explain how this Postgres type can be converted to a Rust type.
impl FromSql<PgGrayType, Pg> for GrayType {
    fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
        let (intensity, suggestion) = FromSql::<PgGrayType, Pg>::from_sql(bytes)?;
        Ok(GrayType {
            intensity,
            suggestion,
        })
    }
}

fn main() {
    let connection = &mut establish_connection();
    // Experiment 1: Define a type for clearer re-use,
    // similar as in the coordinates example.
    type Color = (i32, i32, i32, i32, Option<String>);
    let results: Vec<Color> = colors
        .select((color_id, red, green, blue, color_name))
        .load(connection)
        .expect("Error loading colors");
    for r in results {
        println!(
            "index {:?}, red {:?}, green {:?}, blue {:?}, name: {:?}",
            r.0, r.1, r.2, r.3, r.4
        );
    }
    // Experiment 2: When recognizing the new type with named fields,
    // the code is more readable.
    let results: Vec<(i32, GrayType)> = colors
        .select((color_id, color2grey(red, green, blue)))
        .load(connection)
        .expect("Error loading gray conversions");
    for (i, g) in results {
        println!(
            "Color {:?} has intensity level {:?} with suggested name {:?}",
            i, g.intensity, g.suggestion
        );
    }
    // Experiment 3: Similar, using the type also in the above listed
    // define_sql_function!(...) definition.
    let results: Vec<(i32, GrayType)> = colors
        .select((color_id, color2gray(red, green, blue)))
        .load(connection)
        .expect("Error loading gray conversions");
    for (i, g) in results {
        println!(
            "Color {:?} has intensity level {:?} with suggested name {:?}",
            i, g.intensity, g.suggestion
        );
    }
}
