#[macro_use] extern crate rocket;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use rocket::fs::FileServer;
use rocket::response::Redirect;
use rocket::serde::{Serialize, json::Json};
use rocket::State;
use rocket_dyn_templates::handlebars::TemplateFileError::TemplateError;
use rocket_dyn_templates::Template;
use crate::AppStatus::{Ready, Shutdown};

#[get("/")]
fn index() -> Template {
    let context = Answers { answers: vec!["Bla".to_string(), "Blubb".to_string()], correct: -1 };
    Template::render("index", &context)
}

#[derive(Serialize)]
struct Answers {
    answers: Vec<String>,
    correct: i32
}

#[derive(PartialEq, Serialize, Copy, Clone)]
enum AppStatus {
    Shutdown,
    AnswerPending,
    Ready
}

#[derive(Serialize)]
struct GameState {
    time_left: i32,
    index: i32,
    status: AppStatus
}

#[get("/answers")]
fn get_answers() -> Json<Answers> {
    Json(Answers { answers: vec!["Bla".to_string(), "Blubb".to_string()], correct: -1 })
}

#[post("/press/<ans>")]
fn select_answer(ans: &str) -> Redirect {
    println!("Answered {}", ans);
    Redirect::to(uri!(index()))
}

#[get("/get_state")]
fn get_state(state: &State<Arc<Mutex<GameState>>>) -> Json<GameState> {
    let s = state.lock().unwrap();
    Json(GameState{time_left: s.time_left, index: s.index, status: s.status})
}

#[rocket::main]
async fn main() {
    // objekt erstellen
    let gamestate = Arc::new(
        Mutex::new(GameState { status: Ready, time_left: 0, index: 0 }));

    let g = gamestate.clone();
    thread::spawn(move || {run(g)});
    // thread starten, objekt übergeben

    // + .manage(objekt)
    rocket::build()
        .mount("/", routes![index, get_answers, select_answer, get_state])
        .mount("/static", FileServer::from("static"))
        .manage(gamestate)
        .attach(Template::fairing())
        .launch()
        .await.unwrap();

    // thread join
    println!("ende");
}

fn run(state: Arc<Mutex<GameState>>) {
    loop {
        let mut s = state.lock().unwrap();
        if s.status == Shutdown { break;}
        if s.time_left > 0 {
            s.time_left -= 1;
        }
        if s.time_left <= 0 {
            // time ran out, make new answers
            s.index += 1;
            s.time_left = 10;
        }
        drop(s);
        thread::sleep(Duration::from_secs(1));
    }
}

// für api abfragen (spotify) reqwest
