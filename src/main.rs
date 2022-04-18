#[macro_use]
extern crate rocket;

use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use rocket::fs::FileServer;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;
use crate::AppStatus::{Ready};
use crate::game::{AnswerFromUser, AppStatus, GameState, GameReferences, GameCommand, GamePreferences, ScoreMode};
use std::time::{SystemTime, UNIX_EPOCH};
use rspotify::{AuthCodeSpotify, Credentials, OAuth, scopes};
use rspotify::clients::OAuthClient;

mod game;
mod quiz;
// mod spotify;
mod types;

//---------------------------------------------- POST Routes -----------------------------------------------------------

#[post("/press_button", format = "json", data = "<answer>")]
fn select_answer(state: &State<Arc<Mutex<GameState>>>, answer: Json<AnswerFromUser>) -> Json<GameState> {
  let mut s = state.lock().unwrap();
  if let Err(err) = s.give_answer(answer.into_inner()) {
    eprintln!("Error on giving answer: {:?}", err);
  }
  Json(s.clone())
}

#[post("/start_game")]
fn start_game(references: &State<Arc<Mutex<GameReferences>>>) {
  let r = references.lock().unwrap();
  r.tx_commands.send(GameCommand::StartGame);
}

#[post("/authorize_spotify?<code>")]
fn authorize_spotify(references: &State<Arc<Mutex<GameReferences>>>, code: Option<String>) {
  let mut r = references.lock().unwrap();
  println!("received spotify auth code: {:?}", code);
  if let Some(c) = code {
    let result = r.spotify_client.request_token(c.as_str());
    match result {
      Ok(_) => println!("Got auth token!"),
      Err(e) => eprintln!("Could not get auth token {:?}", e)
    }
  }
}

#[post("/set?<scoremode>&<playlist>&<time_to_answer>&<time_between_answers>&<time_before_round>")]
fn set_preference(preferences: &State<Arc<Mutex<GamePreferences>>>,
                  scoremode: Option<ScoreMode>,
                  playlist: Option<String>,
                  time_to_answer: Option<u32>,
                  time_between_answers: Option<u32>,
                  time_before_round: Option<u32>)
                  -> Json<GamePreferences> {
  let mut p = preferences.lock().unwrap();
  if let Some(sm) = scoremode {
    println!("set scoremode to {:?}", sm);
    p.scoremode = sm;
  }
  if let Some(pl) = playlist {
    println!("set playlist to {}", pl);
    p.selected_playlist = pl;
  }
  if let Some(t) = time_to_answer {
    println!("set time_to_answer to {}", t);
    p.time_to_answer = t;
  }
  if let Some(t) = time_between_answers {
    println!("set time_between_answers to {}", t);
    p.time_between_answers = t;
  }
  if let Some(t) = time_before_round {
    println!("set time_before_round to {}", t);
    p.time_before_round = t;
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
  let gamestate = Arc::new(Mutex::new(GameState::new()));
  let (tx, rx) = mpsc::channel::<GameCommand>();
  let creds = Credentials::from_env().expect("Credentials not in .env-File");
  let references = Arc::new(Mutex::new(
    GameReferences {
      tx_commands: tx,
      spotify_client: AuthCodeSpotify::new(creds,
                                           OAuth::from_env(
                                             scopes!("user-modify-playback-state",
                                               "user-read-playback-state",
                                               "user-read-currently-playing",
                                               "playlist-read-collaborative",
                                               "playlist-read-private",
                                               "app-remote-control",
                                               "streaming",
                                               "user-read-email",
                                               "user-read-private"))
                                             .expect("Credentials not in .env-File")) }));
  let preferences = Arc::new(Mutex::new(GamePreferences::new()));
  let g = gamestate.clone();
  let p = preferences.clone();
  let r = references.clone();
  let handle = thread::spawn(move || { game::run(g, rx, p, r) });

  // Start HTTP interface
  rocket::build()
    .mount("/", routes![select_answer,
            get_state, get_time, start_game, stop_game, set_preference, get_preferences, set_preferences, authorize_spotify])
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
