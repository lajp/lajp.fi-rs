#![feature(once_cell)]

use actix_files::Files;
use actix_web::{error, middleware, web, App, Error, HttpResponse, HttpServer, Result};
use chrono::NaiveDate;
use regex::Regex;
use serde_derive::Serialize;
use std::sync::LazyLock;
use tera::Tera;

static TITLE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\{%\sblock\stitle\s%\}(.*)\{%\sendblock"#).unwrap());
static DATE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\{%\sblock\sdate\s%\}(.*)\{%\sendblock"#).unwrap());
static DESCRIPTION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\{%\sblock\sdescription\s%\}(.*)\{%\sendblock"#).unwrap());

#[macro_use]
extern crate actix_web;

#[derive(Serialize, Debug, Clone)]
pub struct BlogEntry {
    title: String,
    description: String,
    date: String,
    path: String,
}

impl BlogEntry {
    fn new(template: &str) -> Self {
        let title = if let Some(m) = TITLE_REGEX.captures(template).unwrap().get(1) {
            m.as_str().to_string()
        } else {
            String::new()
        };

        let date = if let Some(m) = DATE_REGEX.captures(template).unwrap().get(1) {
            m.as_str().to_string()
        } else {
            String::new()
        };

        let description = if let Some(m) = DESCRIPTION_REGEX.captures(template).unwrap().get(1) {
            m.as_str().to_string()
        } else {
            String::new()
        };

        Self {
            title,
            date,
            description,
            path: String::new(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct BlogContext {
    blogentries: Vec<BlogEntry>,
}

impl BlogContext {
    fn new(path: &str) -> Self {
        let mut entries = Vec::new();

        for file in std::fs::read_dir(std::path::Path::new(path)).unwrap() {
            let content = std::fs::read_to_string(file.as_ref().unwrap().path()).unwrap();
            let mut entry = BlogEntry::new(&content);
            entry.path = format!(
                "/blog/{}",
                file.as_ref().unwrap().file_name().to_str().unwrap()
            );
            entries.push(entry);
        }

        entries.sort_by_key(|k| NaiveDate::parse_from_str(&k.date, "%F").unwrap());
        entries.reverse();

        Self {
            blogentries: entries,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Image {
    path: String,
    name: String,
}

impl Image {
    fn new(path: &str) -> Self {
        Self {
            name: if let Some(idx) = path.rfind('/') {
                path[idx + 1..].to_string()
            } else {
                String::new()
            },
            path: path[1..].to_string(), // Strip the redundant "." from the start
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ImageGallery {
    path: String,
    images: Vec<Image>,
}

impl ImageGallery {
    fn new(path: &str) -> Self {
        let mut images = Vec::new();

        std::fs::read_dir(std::path::Path::new(path))
            .unwrap()
            .for_each(|file| {
                if let Ok(f) = file {
                    if let Some(path) = f.path().to_str() {
                        let image = Image::new(path);
                        images.push(image);
                    }
                }
            });

        Self {
            path: path.to_string(),
            images,
        }
    }
}

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

#[get("/robots.txt")]
async fn robots() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body(std::fs::read_to_string("static/robots.txt")?))
}

#[get("/humans.txt")]
async fn humans() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body(std::fs::read_to_string("static/humans.txt")?))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let blogcontext = web::Data::new(BlogContext::new("./templates/blog/"));
    let imagegallery = web::Data::new(ImageGallery::new("./static/gallery/"));

    HttpServer::new(move || {
        let tera = Tera::new("templates/**/*").unwrap();

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
            .service(robots)
            .service(humans)
            .service(pages)
            .service(blogarticle)
    })
    .bind(("127.0.0.1", 6900))?
    .run()
    .await
}
