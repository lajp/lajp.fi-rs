#![feature(once_cell)]

mod models;

use actix_files::Files;
use actix_multipart::Multipart;
use actix_web::{error, guard, middleware, web, App, Error, HttpResponse, HttpServer, Result};
use futures_util::stream::StreamExt as _;
use models::*;
use std::fs::File;
use std::io::Write;
use tera::Tera;

#[macro_use]
extern crate actix_web;

#[get("/")]
async fn index(tmpl: web::Data<tera::Tera>) -> Result<HttpResponse, Error> {
    let res = tmpl
        .render("index.html", &tera::Context::new())
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(res))
}

#[get("/{page}")]
async fn pages(
    tmpl: web::Data<tera::Tera>,
    path: web::Path<(String,)>,
) -> Result<HttpResponse, Error> {
    let res = tmpl
        .render(
            &format!("{}.html", &path.0.trim_end_matches(".html")),
            &tera::Context::new(),
        )
        .map_err(|_| error::ErrorNotFound("No such page"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(res))
}

#[get("/blog")]
async fn blogindex(
    tmpl: web::Data<tera::Tera>,
    blogcontext: web::Data<BlogContext>,
) -> Result<HttpResponse, Error> {
    let res = tmpl
        .render(
            "blogindex.html",
            &tera::Context::from_serialize(&blogcontext).unwrap(),
        )
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(res))
}

#[get("/blog/{article}")]
async fn blogarticle(
    tmpl: web::Data<tera::Tera>,
    path: web::Path<(String,)>,
) -> Result<HttpResponse, Error> {
    let res = tmpl
        .render(
            &format!("blog/{}.html", &path.0.trim_end_matches(".html")),
            &tera::Context::new(),
        )
        .map_err(|_| error::ErrorNotFound("No such article"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(res))
}

#[post("/gallery")]
async fn add_to_gallery(mut payload: Multipart) -> Result<HttpResponse, Error> {
    let mut filename = String::new();
    let mut content = Vec::<u8>::new();

    while let Some(item) = payload.next().await {
        let mut field = item?;
        if field.name() == "filename" {
            while let Some(chunk) = field.next().await {
                filename.push_str(std::str::from_utf8(&chunk?)?)
            }
        }
        if field.name() == "content" {
            while let Some(chunk) = field.next().await {
                content.append(&mut chunk?.to_vec());
            }
        }
    }

    if !(filename.is_empty() || content.is_empty()) {
        let mut newfile = File::create(format!("./static/gallery/{}", &filename))?;
        newfile.write_all(content.as_slice())?;
        return Ok(HttpResponse::Ok().finish());
    }

    Err(error::ErrorBadRequest("You have done stupiding"))
}

#[get("/gallery")]
async fn gallery(
    tmpl: web::Data<tera::Tera>,
    imagegallery: web::Data<ImageGallery>,
) -> Result<HttpResponse, Error> {
    let res = tmpl
        .render(
            "gallery.html",
            &tera::Context::from_serialize(&imagegallery).unwrap(),
        )
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(res))
}

#[get("/{file}.txt")]
async fn txtfiles(path: web::Path<(String,)>) -> Result<HttpResponse, Error> {
    let content = std::fs::read_to_string(format!("static/{}.txt", &path.0))
        .map_err(|_| error::ErrorNotFound("This, I do not have :/"))?;
    Ok(HttpResponse::Ok().content_type("text/plain").body(content))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let blogcontext = web::Data::new(BlogContext::new("./templates/blog/"));
    let imagegallery = web::Data::new(ImageGallery::new("./static/gallery/"));

    HttpServer::new(move || {
        let tera = Tera::new("templates/**/*").unwrap();

        let galleryauth = format!(
            "Bearer {}",
            std::env::var("GALLERY_TOKEN").expect("NO GALLERY_TOKEN")
        );

        App::new()
            .app_data(web::Data::new(tera))
            .app_data(web::Data::clone(&blogcontext))
            .app_data(web::Data::clone(&imagegallery))
            .wrap(middleware::Logger::new(
                r#"%{r}a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %Dms"#,
            ))
            .service(index)
            .service(Files::new("/static", "./static"))
            .service(blogindex)
            .service(gallery)
            .service(txtfiles)
            .service(pages)
            .service(blogarticle)
            .service(
                web::scope("")
                    .guard(guard::fn_guard(move |ctx| {
                        ctx.head().headers().contains_key("Authorization")
                            && ctx
                                .head()
                                .headers()
                                .get("Authorization")
                                .unwrap()
                                .to_str()
                                .unwrap()
                                == galleryauth
                    }))
                    .service(add_to_gallery),
            )
    })
    .bind(("127.0.0.1", 6900))?
    .run()
    .await
}
