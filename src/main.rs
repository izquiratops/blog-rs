use actix_files::Files;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder, Result};
use ignore::{types::TypesBuilder, WalkBuilder};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::{collections::HashMap, fs, io::Error};
use tera::{to_value, try_get_value, Result as TeraResult, Tera, Value};

extern crate markdown;

#[derive(Deserialize, Serialize)]
pub struct ArticleData {
    title: String,
    file_name: String,
    posted: String,
    hidden: bool,
}

pub fn markdown_filter(value: &Value, _: &HashMap<String, Value>) -> TeraResult<Value> {
    let str = try_get_value!("markdown", "value", String, value);
    let html_content = markdown::to_html(&str);

    Ok(to_value(&html_content).unwrap())
}

pub fn fetch_markdown(post_name: &String) -> Result<String, Error> {
    let path = format!("blog/{}/post.md", post_name);
    fs::read_to_string(path)
}

pub fn fetch_article_data(post_name: &String) -> Result<ArticleData, Error> {
    let path = format!("blog/{}/data.json", post_name);
    let content = fs::read_to_string(path).unwrap();

    Ok(from_str(&content).unwrap())
}

pub fn walk_blog_directory() -> Result<Vec<ArticleData>, Error> {
    let mut t = TypesBuilder::new();
    t.add_defaults();

    let json = t
        .select("json")
        .build()
        .expect("Couldn't build json file type matcher");

    let file_walker = WalkBuilder::new("blog").types(json).build();

    let mut article_data_list: Vec<ArticleData
> = vec![];
    for directory_entry in file_walker {
        match directory_entry {
            Ok(entry) => {
                if entry.path().is_file() {
                    let content = fs::read_to_string(entry.path())?;
                    let article_data_content = serde_json::from_str(&content)?;
                    article_data_list.push(article_data_content);
                }
            }
            Err(e) => panic!("{}", e),
        }
    }

    Ok(article_data_list)
}

#[get("/blog/{article_name}")]
async fn blog_article(tmpl: web::Data<tera::Tera>, article_name: web::Path<String>) -> impl Responder {
    let mut ctx = tera::Context::new();

    let markdown_content = match fetch_markdown(&article_name) {
        Ok(s) => s,
        Err(_e) => {
            return HttpResponse::NotFound()
                .content_type("text/html")
                .body("<p>Could not find post, sorry!</p>");
        }
    };

    let article_data = match fetch_article_data(&article_name) {
        Ok(s) => s,
        Err(_e) => {
            return HttpResponse::NotFound()
                .content_type("text/html")
                .body("<p>Could not find post, sorry!</p>");
        }
    };

    ctx.insert("markdown_content", &markdown_content);
    ctx.insert("article_data", &article_data);

    match tmpl.render("blog_article.html", &ctx) {
        Ok(s) => HttpResponse::Ok().content_type("text/html").body(s),
        Err(_e) => {
            return HttpResponse::NotFound()
                .content_type("text/html")
                .body("<p>I'm struggling with the templates 💩</p>")
        }
    }
}

#[get("/")]
async fn index(
    tmpl: web::Data<tera::Tera>,
    _query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let mut ctx = tera::Context::new();

    let article_data_list = walk_blog_directory().unwrap();

    ctx.insert("article_data_list", &article_data_list);

    match tmpl.render("index.html", &ctx) {
        Ok(s) => HttpResponse::Ok().content_type("text/html").body(s),
        Err(_e) => {
            return HttpResponse::NotFound()
                .content_type("text/html")
                .body("<p>Template not found</p>")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        let mut tera = Tera::new("templates/**/*.html").expect("Couldn't load the template folder");

        tera.register_filter("markdown", markdown_filter);

        App::new()
            .app_data(web::Data::new(tera))
            .service(Files::new("/static", "./static"))
            .service(index)
            .service(blog_article)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
