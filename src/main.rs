#[macro_use] extern crate rocket;

use std::sync::{Arc, Mutex};
use std::thread;
use rocket::fs::FileServer;
use rocket::serde::{json::Json, Serialize};
use rocket::State;
use rocket_dyn_templates::Template;
use crate::AppStatus::{Ready};
use crate::game::{AnswerFromUser, AppStatus, GameState};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod game;


#[post("/press_button", format = "json", data = "<answer>")]
fn select_answer(state: &State<Arc<Mutex<GameState>>>, answer: Json<AnswerFromUser>) {
    let mut s = state.lock().unwrap();
    if let Err(err)  = s.give_answer(answer.into_inner()) {
        println!("Error on giving answer: {}", err);
    }
}

#[get("/get_state")]
fn get_state(state: &State<Arc<Mutex<GameState>>>) -> Json<GameState> {
    let s = state.lock().unwrap();
    Json(s.clone())
}

#[derive(Serialize)]
struct TimeAnswer {
    diff_receive: i64,
    ts: u64
}

#[get("/get_time?<now>")]
fn get_time(now: Option<u64>) -> Json<TimeAnswer> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is < UNIX_EPOCH")
        .as_millis() as u64;
    match now {
        Some(now) => Json(TimeAnswer{diff_receive: now as i64 - now_ms as i64, ts: now_ms}),
        None => Json(TimeAnswer{diff_receive: 0, ts: now_ms})
    }
}

#[rocket::main]
async fn main() {
    // objekt erstellen

    let gamestate = Arc::new(
        Mutex::new(GameState::new()));

    let g = gamestate.clone();
    let handle = thread::spawn(move || {game::run(g)});
    // thread starten, objekt übergeben

    // + .manage(objekt)
    rocket::build()
        .mount("/", routes![select_answer, get_state, get_time])
        .mount("/static", FileServer::from("static"))
        .manage(gamestate)
        .launch()
        .await.unwrap();

    handle.join().unwrap();
    // thread join
    println!("ende");
}



// für api abfragen (spotify) reqwest
