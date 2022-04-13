#[macro_use]
extern crate rocket;

use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use rocket::fs::FileServer;
use rocket::serde::{json::Json, Serialize};
use rocket::State;
use crate::AppStatus::{Ready};
use crate::game::{AnswerFromUser, AppStatus, GameState, GameReferences, GameCommand, GamePreferences, ScoreMode};
use std::time::{SystemTime, UNIX_EPOCH};

mod game;

//---------------------------------------------- POST Routes -----------------------------------------------------------

#[post("/press_button", format = "json", data = "<answer>")]
fn select_answer(state: &State<Arc<Mutex<GameState>>>, answer: Json<AnswerFromUser>) -> Json<GameState> {
  let mut s = state.lock().unwrap();
  if let Err(err) = s.give_answer(answer.into_inner()) {
    eprintln!("Error on giving answer: {}", err);
  }
  Json(s.clone())
}

#[post("/start_game?<playlist>")]
fn start_game(references: &State<Arc<Mutex<GameReferences>>>, playlist: Option<String>) {
  let r = references.lock().unwrap();
  if let Some(p) = playlist {
    match r.tx_commands.send(GameCommand::StartGame(p.clone())) {
      Err(e) => eprintln!("Error on starting game with playlist {}: {}", p, e),
      Ok(_) => println!("Started game, playlist = {}", p)
    }
  }
}

#[post("/set?<scoremode>")]
fn set_preference(preferences: &State<Arc<Mutex<GamePreferences>>>, scoremode: Option<ScoreMode>)
                  -> Json<GamePreferences> {
  let mut p = preferences.lock().unwrap();
  if let Some(sm) = scoremode {
    p.scoremode = sm;
    println!("set scoremode to {}", sm); // todo
  }
  Json(p.clone())
}

#[post("/set_preferences", format = "json", data = "<received>")]
fn set_preferences(preferences: &State<Arc<Mutex<GamePreferences>>>, received: Json<GamePreferences>)
                   -> Json<GamePreferences> {
  let mut p = preferences.lock().unwrap();
  *p = received.into_inner();
  Json(p.clone())
}

#[post("/stop_game")]
fn stop_game(references: &State<Arc<Mutex<GameReferences>>>) {
  let r = references.lock().unwrap();
  match r.tx_commands.send(GameCommand::StopGame) {
    Err(e) => eprintln!("Game could not be stopped: {}", e),
    Ok(_) => println!("Stopped game")
  }
}

//----------------------------------------------- GET Routes -----------------------------------------------------------

#[get("/get_state")]
fn get_state(state: &State<Arc<Mutex<GameState>>>) -> Json<GameState> {
  let s = state.lock().unwrap();
  Json(s.clone())
}


#[get("/get_preferences")]
fn get_preferences(preferences: &State<Arc<Mutex<GamePreferences>>>)
                   -> Json<GamePreferences> {
  let p = preferences.lock().unwrap();
  Json(p.clone())
}


#[derive(Serialize)]
struct TimeAnswer {
  diff_receive: i64,
  ts: u64,
}

#[get("/get_time?<now>")]
fn get_time(now: Option<u64>) -> Json<TimeAnswer> {
  let now_ms = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("System time is < UNIX_EPOCH")
    .as_millis() as u64;
  match now {
    Some(now) => Json(TimeAnswer { diff_receive: now as i64 - now_ms as i64, ts: now_ms }),
    None => Json(TimeAnswer { diff_receive: 0, ts: now_ms })
  }
}

#[rocket::main]
async fn main() {
  // objekt erstellen

  let gamestate = Arc::new(
    Mutex::new(GameState::new()));
  let (tx, rx) = mpsc::channel::<GameCommand>();
  let references = Arc::new(Mutex::new(
    GameReferences { spotify_handle: 0, tx_commands: tx }));
  let preferences = Arc::new(Mutex::new(
    GamePreferences::new()));
  let g = gamestate.clone();
  let p = preferences.clone();
  let handle = thread::spawn(move || { game::run(g, rx, p) });
  // thread starten, objekt übergeben

  // HTTP interface starten
  rocket::build()
    .mount("/", routes![select_answer,
            get_state, get_time, start_game, stop_game, set_preference, get_preferences, set_preferences])
    .mount("/static", FileServer::from("static"))
    .manage(gamestate)
    .manage(references)
    .manage(preferences)
    .launch()
    .await.unwrap();


  handle.join().unwrap();
  // thread join
  println!("ende");
}


// für api abfragen (spotify) reqwest
