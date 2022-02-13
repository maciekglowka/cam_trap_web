#[macro_use] extern crate rocket;

use std::cmp::min;
use rocket::State;
use rocket::response::Redirect;
use rocket::request::{FromRequest, Request, Outcome};
use rocket::fs::FileServer;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::form::Form;
use rocket_dyn_templates::Template;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::io;

#[derive(Deserialize)]
struct Settings {
    media_dir: String,
    password: String,
    display_count: usize,
}

#[derive(FromForm)]
struct LoginForm<'a> {
    password: &'a str
}

struct User<'a> (&'a str);

#[rocket::async_trait]
impl<'a> FromRequest<'a> for User<'a> {
    type Error = ();

    async fn from_request(request: &'a Request<'_>) -> Outcome<Self, Self::Error> {
        let pass = request.cookies().get_private("password");

        match pass {
            None => Outcome::Forward(()),
            Some(cookie) => {
                let settings = request.guard::<&State<Settings>>().await.unwrap();
                if cookie.value() == settings.password { Outcome::Success(User("pass")) }
                else {Outcome::Failure((Status::Forbidden, ()))}
            }
        }
    }
}

#[get("/")]
fn index(settings: &State<Settings>, user: User) -> Template {
    let mut file_paths: Vec::<String> = fs::read_dir(&settings.media_dir)
        .expect("Can't read media_dir")
        .filter_map(|f| f.ok())
        .map(|f| f.file_name().to_str().unwrap().to_owned()).collect::<Vec<String>>();

    file_paths.sort_by(|a, b| b.cmp(a));

    let idx = min(settings.display_count, file_paths.len());
    let context = json!({"image_paths": file_paths[..idx]});
    Template::render("index", &context)
}

#[get("/", rank=2)]
fn no_auth_index() -> Redirect {
    Redirect::to(uri!(login_get))
}

#[get("/remove?<path>")]
fn remove(settings: &State<Settings>, user:User, path: &str) -> Redirect {
    let file_path = format!("{}/{}", settings.media_dir, path);
    fs::remove_file(file_path).unwrap();
    Redirect::to(uri!(index))
}

#[get("/login")]
fn login_get() -> Template {
    Template::render("login", json!({}))
}

#[post("/login", data = "<login>")]
fn login_post(jar: &CookieJar<'_>, login: Form<LoginForm<'_>>) -> Redirect { ;
    jar.add_private(Cookie::new("password", login.password.to_owned()));
    Redirect::to(uri!(index))
}

#[post("/logout")]
fn logout(jar: &CookieJar<'_>) -> Redirect { ;
    jar.remove_private(Cookie::named("password"));
    Redirect::to(uri!(login_get))
}

#[launch]
fn rocket() -> _ {
    let file = fs::File::open("settings.json").expect("Settings file not found!");
    let reader = io::BufReader::new(file);
    let settings: Settings = serde_json::from_reader(reader).expect("Settings cannot be parsed!");

    let media_dir = &settings.media_dir.clone();

    rocket::build()
        .manage(settings)
        .mount("/", routes![index, login_get, login_post, logout, no_auth_index, remove])
        .mount("/media", FileServer::from(media_dir))
        .mount("/static", FileServer::from("static/"))
        .attach(Template::fairing())
}