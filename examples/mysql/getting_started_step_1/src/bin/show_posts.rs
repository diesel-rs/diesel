use self::models::*;
use diesel::prelude::*;
use diesel_demo_step_1_mysql::*;

fn main() {
    use self::schema::posts::dsl::*;
    use self::schema::companys::dsl::*;

    let connection = establish_connection();
    let results = posts
        .filter(published.eq(true))
        .limit(5)
        .load::<Post>(&connection)
        .expect("Error loading posts");

    println!("Displaying {} posts", results.len());
    for post in results {
        println!("{}", post.title);
        println!("-----------\n");
        println!("{}", post.body);
    }

    // let results = companys
    //     .filter(company_code.eq("O0000001"))        
    //     .load::<Company>(&connection)
    //     .expect("Error loading posts");

    // println!("Displaying {} posts", results.len());
    // for company in results {
    //     println!("{}", company.company_code);
    //     println!("-----------\n");
    //     println!("{}", company.company_name);
    // }
}
