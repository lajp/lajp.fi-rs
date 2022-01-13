mod database;
pub mod models;
pub mod schema;

use rocket::fs::FileServer;
use rocket::State;
use rocket_dyn_templates::Template;
use serde_json::json;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate diesel;

#[get("/")]
async fn index() -> Template {
    Template::render("index", json!({}))
}

#[get("/<page>")]
async fn pages(page: String) -> Template {
    Template::render(page, json!({}))
}

#[get("/blog")]
async fn blogindex(database: &State<database::Database>) -> Template {
    let blogentries = models::BlogContext {
        blogentries: database.get_blog_entries(),
    };
    Template::render("blogindex", &blogentries)
}

#[get("/<article>")]
async fn blogarticle(article: String) -> Template {
    Template::render(format!("blog/{}", article), json!({}))
}

#[launch]
fn rocket() -> _ {
    dotenv::dotenv().ok();
    let database = database::Database::new();
    database.check_for_new_entries();
    rocket::build()
        .mount("/static", FileServer::from("static"))
        .mount("/", routes![index])
        .mount("/", routes![pages])
        .mount("/", routes![blogindex])
        .mount("/blog", routes![blogarticle])
        .attach(Template::fairing())
        .manage(database)
}
