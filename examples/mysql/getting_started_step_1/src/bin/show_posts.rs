use self::models::*;
use diesel::prelude::*;
use diesel_demo_step_1_mysql::*;

fn main() {
    use self::schema::posts::dsl::*;
    use self::schema::company::dsl::*;

    let connection = &mut establish_connection();
    let results = posts
        .filter(published.eq(true))
        .limit(5)
        .load::<Post>(connection)
        .expect("Error loading posts");

    let post = Post{
        id : 3,
        title : "test".to_string(),
        body : "body".to_string(),
        published : true,        
    };

    diesel::insert_into(posts).values(post).execute(connection).expect("insert error");
        

    println!("Displaying {} posts", results.len());
    for post in results {
        println!("{}", post.title);
        println!("-----------\n");
        println!("{}", post.body);
    }

    let post = Post{
                id : 1,
                title : "test".to_string(),
                body : "body".to_string(),
                published : true,
     };
    diesel::insert_into(posts).values(post).execute(connection).expect("insert error");
    

    let results = company
        .filter(CompanyID.eq(1))              
        .load::<Company>(connection)
        .expect("Error loading company");

    println!("Displaying {} company", results.len());
    for company1 in results {
        println!("CompanyID:{}", company1.CompanyID);
        println!("CompanyCode:{}", company1.CompanyCode);
        println!("CompanyName:{}", company1.CompanyName);
        // println!("Create Date:{}", company1.DateCreated);
    }
}
