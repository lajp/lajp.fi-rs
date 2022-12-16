#![feature(once_cell)]

mod models;
mod payloadverifier;

use crate::models::*;
use actix_files::Files;
use actix_multipart::Multipart;
use actix_rt::time;
use actix_web::{dev, error, guard, middleware, web, App, Error, HttpResponse, HttpServer, Result};
use futures_util::stream::StreamExt as _;
use hmac::{Hmac, Mac};
use rand::distributions::Alphanumeric;
use rand::Rng;
use sha2::Sha256;
use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::sync::Mutex;
use tera::Tera;

#[macro_use]
extern crate actix_web;

async fn update(
    payload: web::Json<UpdatePayload>,
    tmpl: web::Data<Mutex<Tera>>,
    blogcontext: web::Data<Mutex<BlogContext>>,
) -> Result<HttpResponse, Error> {
    Command::new("git").arg("pull").output().unwrap();

    if let Some(workflow) = &payload.workflow_run {
        let token = std::env::var("GITHUB_TOKEN").expect("No GITHUB_TOKEN");
        let client = reqwest::Client::builder()
            .user_agent("balls") // required by github
            .build()
            .unwrap();

        let res = client
            .get(&workflow.artifacts_url)
            .header("Authorization", format!("token {}", &token))
            .send()
            .await
            .unwrap();

        let artifacts_payload = res.json::<Artifacts>().await.unwrap();

        if !artifacts_payload.artifacts.is_empty() {
            let download_url = &artifacts_payload.artifacts[0].archive_download_url;

            let res = client
                .get(download_url)
                .header("Authorization", format!("token {}", &token))
                .send()
                .await
                .unwrap();

            let mut zipfile = File::create("build.zip")?;
            let mut content = std::io::Cursor::new(res.bytes().await.unwrap());
            std::io::copy(&mut content, &mut zipfile).unwrap();

            Command::new("unzip")
                .args(["-o", "build.zip"])
                .output()
                .unwrap();

            Command::new("chmod")
                .args(["+x", "lajp_fi-rs"])
                .output()
                .unwrap();

            std::thread::spawn(|| {
                // A very, very cursed way of restarting
                std::thread::sleep(std::time::Duration::from_secs(2));
                std::process::exit(1);
            });
            return Ok(HttpResponse::Ok().body("Update done! Restarting now!"));
        }
    }

    tmpl.lock().unwrap().full_reload().unwrap();
    blogcontext.lock().unwrap().reload();

    Ok(HttpResponse::Ok().body("Update done! Reloading files now!"))
}

async fn get_activity(a: &web::Data<Mutex<Option<Activity>>>) {
    let res = reqwest::Client::new()
        .get("https://api.testaustime.fi/users/@me/activity/current")
        .bearer_auth(std::env::var("TESTAUSTIME_TOKEN").expect("No TESTAUSTIME_TOKEN"))
        .send()
        .await
        .unwrap();

    if let Ok(na) = res.json::<Activity>().await {
        if let Ok(mut l) = a.try_lock() {
            *l = Some(na);
        }
    }
}

#[get("/")]
async fn index(
    tmpl: web::Data<Mutex<Tera>>,
    activity: web::Data<Mutex<Option<Activity>>>,
) -> Result<HttpResponse, Error> {
    let a = activity.lock().unwrap();
    let index_activity = IndexActivity::from(a.clone());

    let res = tmpl
        .lock()
        .unwrap()
        .render(
            "index.html",
            &tera::Context::from_serialize(&index_activity).unwrap(),
        )
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(res))
}

#[get("/{page}")]
async fn pages(
    tmpl: web::Data<Mutex<Tera>>,
    path: web::Path<(String,)>,
) -> Result<HttpResponse, Error> {
    let res = tmpl
        .lock()
        .unwrap()
        .render(
            &format!("{}.html", &path.0.trim_end_matches(".html")),
            &tera::Context::new(),
        )
        .map_err(|_| error::ErrorNotFound("No such page"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(res))
}

#[get("/blog")]
async fn blogindex(
    tmpl: web::Data<Mutex<Tera>>,
    blogcontext: web::Data<Mutex<BlogContext>>,
) -> Result<HttpResponse, Error> {
    let res = tmpl
        .lock()
        .unwrap()
        .render(
            "blogindex.html",
            &tera::Context::from_serialize(&*blogcontext.lock().unwrap()).unwrap(),
        )
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(res))
}

#[get("/blog/{article}")]
async fn blogarticle(
    tmpl: web::Data<Mutex<Tera>>,
    path: web::Path<(String,)>,
) -> Result<HttpResponse, Error> {
    let res = tmpl
        .lock()
        .unwrap()
        .render(
            &format!("blog/{}.html", &path.0.trim_end_matches(".html")),
            &tera::Context::new(),
        )
        .map_err(|_| error::ErrorNotFound("No such article"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(res))
}

#[post("/gallery")]
async fn add_to_gallery(
    mut payload: Multipart,
    imagegallery: web::Data<Mutex<ImageGallery>>,
) -> Result<HttpResponse, Error> {
    let mut filename = String::new();
    let mut content = Vec::<u8>::new();

    while let Some(item) = payload.next().await {
        let mut field = item?;
        if field.name() == "file" {
            let cd = field.content_disposition();
            filename = cd.get_filename().unwrap().to_string();

            while let Some(chunk) = field.next().await {
                content.append(&mut chunk?.to_vec());
            }
        }
    }

    if !content.is_empty() {
        let path = format!("./static/gallery/{}", &filename);
        let mut img = Image::new(&path);

        while imagegallery.lock().unwrap().images.contains(&img) {
            let mut ext = format!(".{}", &img.name.rsplit('.').next().unwrap_or_default());
            if ext.len() == 1 {
                ext.clear();
            }

            let c = rand::thread_rng().sample(Alphanumeric) as char;
            img.path.truncate(img.path.len() - ext.len());
            img.name.truncate(img.name.len() - ext.len());
            img.path.push(c);
            img.path.push_str(&ext);
            img.name.push(c);
            img.name.push_str(&ext);
        }

        let mut newfile = File::create(format!(".{}", &img.path))?;
        newfile.write_all(content.as_slice())?;

        imagegallery.lock().unwrap().add_image(img);

        return Ok(HttpResponse::Ok().finish());
    }

    Err(error::ErrorBadRequest("You have done stupiding"))
}

#[get("/gallery")]
async fn gallery(
    tmpl: web::Data<Mutex<Tera>>,
    imagegallery: web::Data<Mutex<ImageGallery>>,
) -> Result<HttpResponse, Error> {
    imagegallery.lock().unwrap().shuffle();
    let res = tmpl
        .lock()
        .unwrap()
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

#[get("/ip")]
async fn whatsmyip(conn: dev::ConnectionInfo) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body(conn.realip_remote_addr().unwrap_or_default().to_string() + "\n"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let blogcontext = web::Data::new(Mutex::new(BlogContext::new("./templates/blog/")));
    let imagegallery = web::Data::new(Mutex::new(ImageGallery::new("./static/gallery/")));
    let activity: web::Data<Mutex<Option<Activity>>> = web::Data::new(Mutex::new(None));
    let activity_clone = activity.clone();

    actix_rt::spawn(async move {
        let mut interval = time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            get_activity(&activity_clone).await;
        }
    });

    HttpServer::new(move || {
        let tera = Mutex::new(Tera::new("templates/**/*").unwrap());

        let secret = std::env::var("GITHUB_SECRET").expect("No GITHUB_SECRET");
        let sbytes = secret.as_bytes();

        let galleryauth = format!(
            "Bearer {}",
            std::env::var("GALLERY_TOKEN").expect("NO GALLERY_TOKEN")
        );

        App::new()
            .app_data(web::Data::new(tera))
            .app_data(web::Data::clone(&blogcontext))
            .app_data(web::Data::clone(&imagegallery))
            .app_data(web::Data::clone(&activity))
            .service(Files::new("/static", "./static"))
            .service(
                web::scope("")
                    .wrap(middleware::Logger::new(
                        r#"%{r}a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %Dms"#,
                    ))
                    .service(index)
                    .service(blogindex)
                    .service(gallery)
                    .service(txtfiles)
                    .service(whatsmyip)
                    .service(pages)
                    .service(blogarticle)
                    .service(
                        web::resource("/update")
                            .wrap(payloadverifier::PayloadVerifier {
                                mac: Hmac::<Sha256>::new_from_slice(sbytes).unwrap(),
                            })
                            .route(web::post().to(update)),
                    )
                    .service(
                        web::scope("")
                            .guard(guard::Header(
                                "Authorization",
                                Box::leak(galleryauth.into_boxed_str()),
                            ))
                            .service(add_to_gallery),
                    ),
            )
    })
    .bind(("127.0.0.1", 6900))?
    .run()
    .await
}
