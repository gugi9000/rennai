#![feature(proc_macro_hygiene, decl_macro, derive_eq)]

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate serde_derive;

use rocket::Request;
use rocket::State;
use rocket_contrib::templates::Template;
use rocket_contrib::{json::Json, serve::StaticFiles};
use rusqlite::{Connection, Error, NO_PARAMS};
use spongedown;
use std::collections::HashMap;
use std::convert::From;
use std::fs;
use std::fs::File;
use std::net::SocketAddr;
use std::sync::Mutex;
use tsv;

use chrono::{Datelike, Timelike, Utc};

type DbConn = Mutex<Connection>;
type TempsMap = HashMap<(String, String), f64>;
type Map = HashMap<String, u32>;

#[derive(Serialize)]
struct TemplateContext {
    name: String,
    section: String,
    items: Vec<&'static str>,
}

#[derive(Debug, Hash, Serialize, PartialEq)]
struct HueTemp {
    sensor: String,
    temperature: String,
    date: String,
}

#[derive(Serialize)]
struct PageContext {
    id: i32,
    name: String,
    section: String,
    release_date: String,
    intro: String,
    contents: String,
}

impl PageContext {
    pub fn from_id(id: i32, connection: &Connection) -> Option<PageContext> {
        connection
            .query_row(
                "SELECT \
                 id, section, release_date, intro, contents \
                 FROM  \
                 WHERE id=?1",
                &[&id],
                |row| PageContext {
                    id: row.get(0),
                    name: row.get(1),
                    section: row.get(2),
                    release_date: row.get(3),
                    intro: row.get(4),
                    contents: row.get(5),
                },
            )
            .ok()
    }
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

#[get("/page/<name>")]
fn page(name: String) -> Template {
    // fn page(name: String) -> String {
    let md = fs::read_to_string("md/uge7-9.md").expect("Something went wrong reading the file");
    let html = spongedown::parse(&md).unwrap();
    let context = PageContext {
        id: 1,
        name,
        section: String::from("page"),
        release_date: String::from("2019-02-03"),
        intro: String::from("Introduction to the page"),
        contents: html,
    };
    // rocket_contrib::templates::tera::Tera::one_off("page", &context, false).unwrap()
    Template::render("page", &context)
}

#[get("/article")]
fn article_index() -> Template {
    let context = TemplateContext {
        name: String::from("Articles"),
        section: String::from("article"),
        items: vec!["One", "Two", "Three"],
    };
    Template::render("articleindex", &context)
}

#[get("/article/<name>")]
fn article(name: String) -> Template {
    let context = TemplateContext {
        name,
        section: String::from("article"),
        items: vec!["Foo", "Bar", "Baz", "Four?"],
    };
    Template::render("article", &context)
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

fn today() -> String {
    let now = Utc::now();
    let (_, year) = now.year_ce();
    format!("{}-{:02}-{:02}", year, now.month(), now.day(),)
}

fn now_24h_ago() -> String {
    let now = Utc::now();
    let (_, year) = now.year_ce();
    format!(
        "{}-{:02}-{:02} {:02}:{:02}:{:02}",
        year,
        now.month(),
        now.day() - 1,
        now.hour(),
        now.minute(),
        now.second(),
    )
}

#[get("/data/temps.json")]
fn temps_json(db_conn: State<DbConn>) -> Result<Json<Vec<HueTemp>>, Error> {
    let query = format!(
        "SELECT name, temp, date FROM registrations where date > '{}' ORDER BY date, name",
        now_24h_ago()
    );

    Ok(Json(
        db_conn
            .lock()
            .expect("db connection lock")
            .prepare(&query)?
            .query_map(NO_PARAMS, |row| HueTemp {
                sensor: row.get(0),
                temperature: row.get(1),
                date: row.get(2),
            })?
            .filter_map(Result::ok)
            .collect(),
    ))
}

#[get("/data/temps.tsv")]
fn temps_tsv(db_conn: State<DbConn>) -> String {
    let query = format!(
        "SELECT name, temp, date FROM registrations where date > '{}' ORDER BY date, name",
        now_24h_ago()
    );

    let tempsmap: TempsMap;

    tempsmap = db_conn
        .lock()
        .expect("db connection lock")
        .prepare(&query)
        .unwrap()
        .query_map(NO_PARAMS, |row| ((row.get(0), row.get(2)), row.get(1)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    let str_repr = tsv::ser::to_string(&tempsmap, tsv::Config::default());

    match str_repr {
        Ok(str_repr) => str_repr,
        _ => "Failed to load data.".to_owned(),
    }
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
fn not_found(req: &Request) -> Template {
    let mut map = HashMap::new();
    map.insert("path", req.uri().path());
    Template::render("error/404", &map)
}

fn rocket() -> rocket::Rocket {
    let database = "huetemps.db";
    let exists = File::open(database).is_ok();
    let conn = Connection::open(database).unwrap();
    if !exists {
        println!("Database didn't exist... creating one");
        conn.execute_batch(include_str!("huetemps.schema")).unwrap();
    }

    rocket::ignite()
        // Have Rocket manage the database pool.
        .manage(Mutex::new(conn))
        .mount("/static", StaticFiles::from("static"))
        .mount(
            "/",
            routes![
                index,
                article_index,
                article,
                page,
                ip,
                ip_json,
                contact,
                temps_json,
                temps_tsv,
            ],
        )
        .attach(Template::fairing())
        .register(catchers![not_found])
}

fn main() {
    rocket().launch();
}
