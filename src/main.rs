#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate serde_derive;

use rocket::{response::content, Request};
use rocket_contrib::templates::Template;
use rocket_contrib::{json::Json, serve::StaticFiles};
use std::{net::SocketAddr, path::PathBuf};

// use std::collections::HashMap;

#[derive(Serialize)]
struct TemplateContext {
    name: String,
    section: String,
    items: Vec<&'static str>,
}

#[get("/")]
fn index() -> Template {
    let context = TemplateContext {
        name: String::from("Home"),
        section: String::from("home"),
        items: vec!["One", "Two", "Three"],
    };
    Template::render("index", &context)
}

#[get("/showroom")]
fn showroom_index() -> Template {
    let context = TemplateContext {
        name: String::from("Showroom"),
        section: String::from("showroom"),
        items: vec!["One", "Two", "Three"],
    };
    Template::render("showroomindex", &context)
}

#[get("/showroom/<name>")]
fn showroom(name: String) -> Template {
    let context = TemplateContext {
        name,
        section: String::from("showroom"),
        items: vec![
            "Foo",
            "Bar",
            "Baz",
            "Four?",
        ],
    };
    Template::render("showroom", &context)
}

#[get("/contact")]
fn contact() -> Template {
    let context = TemplateContext {
        name: String::from("index"),
        section: String::from("contact"),
        items: vec!["One", "Two", "Three"],
    };
    Template::render("contact", &context)
}

#[get("/ip")]
fn ip(addr: SocketAddr) -> String {
    format!("{}\n", addr.ip())
}

#[get("/ip.json")]
fn ip_json(addr: SocketAddr) -> Json<String> {
    let ip = format!("{{ 'ip': '{}' }}", addr.ip());
    Json(ip)
}

#[catch(404)]
fn not_found(req: &Request) -> String {
    format!("Sorry, '{}' is not a valid path.", req.uri())
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/static", StaticFiles::from("static"))
        .mount(
            "/",
            routes![index, showroom_index, showroom, ip, ip_json, contact],
        )
        .attach(Template::fairing())
        .register(catchers![not_found])
}

fn main() {
    rocket().launch();
}
