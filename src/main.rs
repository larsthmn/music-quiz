#[macro_use]
extern crate rocket;

use std::sync::{Arc, mpsc, Mutex};
use std::{fs, thread};
use rocket::fs::FileServer;
use rocket::serde::{json::Json, Serialize};
use rocket::State;
use crate::game::{AnswerFromUser, GameState, GameReferences, GameCommand, GamePreferences, ScoreMode};
use std::time::{SystemTime, UNIX_EPOCH};
use rspotify::{AuthCodeSpotify, Config, Credentials, OAuth, scopes};
use rspotify::clients::{BaseClient, OAuthClient};
use rusqlite::Connection;
use simple_logger::SimpleLogger;
use log::LevelFilter;
use rocket::serde::json::serde_json;
use crate::spotify::{spotify_loop};

mod game;
mod quiz;
// mod spotify;
mod spotify;

const PREFERENCES_FILE: &'static str = "preferences.json";

//---------------------------------------------- POST Routes -----------------------------------------------------------

#[post("/press_button", format = "json", data = "<answer>")]
fn select_answer(state: &State<Arc<Mutex<GameState>>>, answer: Json<AnswerFromUser>) -> Json<GameState> {
  let mut s = state.lock().unwrap();
  if let Err(err) = s.give_answer(answer.into_inner()) {
    log::warn!("Error on giving answer: {:?}", err);
  }
  Json(s.clone())
}

#[post("/start_game")]
fn start_game(references: &State<Arc<Mutex<GameReferences>>>) {
  let r = references.lock().unwrap();
  if let Err(e) =  r.tx_commands.send(GameCommand::StartGame) {
    log::warn!("Could not send game command ({:?})", e)
  }
}

#[post("/authorize_spotify?<code>")]
fn authorize_spotify(references: &State<Arc<Mutex<GameReferences>>>, code: Option<String>) {
  let mut r = references.lock().unwrap();
  log::info!("received spotify auth code: {:?}", code);
  if let Some(c) = code {
    let result = r.spotify_client.request_token(c.as_str());
    match result {
      Ok(_) => log::info!("Got auth token!"),
      Err(e) => log::warn!("Could not get auth token {:?}", e)
    }
  }
}

#[post("/set?<scoremode>&<playlist>&<time_to_answer>&<time_between_answers>&<time_before_round>&<rounds>&<preview_mode>&<hide_answers>")]
fn set_preference(preferences: &State<Arc<Mutex<GamePreferences>>>,
                  scoremode: Option<ScoreMode>,
                  playlist: Option<String>,
                  time_to_answer: Option<u32>,
                  time_between_answers: Option<u32>,
                  time_before_round: Option<u32>,
                  rounds: Option<u32>,
                  preview_mode: Option<bool>,
                  hide_answers: Option<bool>)
                  -> Json<GamePreferences> {
  let mut p = preferences.lock().unwrap();
  if let Some(sm) = scoremode {
    log::info!("set scoremode to {:?}", sm);
    p.scoremode = sm;
  }
  if let Some(id) = playlist {
    if let Some(selected_playlist) = p.playlists.iter().find(|x| x.id == id) {
      log::info!("set playlist to {:?}", selected_playlist);
      p.selected_playlist = Some(selected_playlist.clone());
    }
  }
  if let Some(t) = time_to_answer {
    log::info!("set time_to_answer to {}", t);
    p.time_to_answer = t;
  }
  if let Some(t) = time_between_answers {
    log::info!("set time_between_answers to {}", t);
    p.time_between_answers = t;
  }
  if let Some(t) = time_before_round {
    log::info!("set time_before_round to {}", t);
    p.time_before_round = t;
  }
  if let Some(r) = rounds {
    log::info!("set rounds to {}", r);
    p.rounds = r;
  }
  if let Some(m) = preview_mode {
    log::info!("set preview_mode to {}", m);
    p.preview_mode = m;
  }
  if let Some(m) = hide_answers {
    log::info!("set hide_answers to {}", m);
    p.hide_answers = m;
  }
  let new_preferences = p.clone();
  drop(p);
  save_preferences(&new_preferences, PREFERENCES_FILE);
  Json(new_preferences)
}

fn save_preferences(new_preferences: &GamePreferences, to: &str) {
  match fs::File::create(to) {
    Ok(file) => match serde_json::to_writer_pretty::<fs::File, GamePreferences>(file, &new_preferences) {
      Ok(_) => log::info!("Saved preferences to file"),
      Err(e) => log::warn!("Could not save preferences to file ({:?})", e)
    },
    Err(e) => log::warn!("Could not open file to write: {:?}", e)
  }
}

#[post("/set_preferences", format = "json", data = "<received>")]
fn set_preferences(preferences: &State<Arc<Mutex<GamePreferences>>>, received: Json<GamePreferences>)
                   -> Json<GamePreferences> {
  let mut p = preferences.lock().unwrap();
  *p = received.into_inner();
  let new_preferences = p.clone();
  drop(p);
  save_preferences(&new_preferences, PREFERENCES_FILE);
  Json(new_preferences)
}

#[post("/stop_game")]
fn stop_game(references: &State<Arc<Mutex<GameReferences>>>) {
  let r = references.lock().unwrap();
  match r.tx_commands.send(GameCommand::StopGame) {
    Err(e) => log::warn!("Game could not be stopped: {}", e),
    Ok(_) => log::info!("Stopped game")
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

fn db_init(con: &Connection) -> rusqlite::Result<()> {
  con.execute("create table if not exists spotify (
                      id integer primary key,
                      token text,
                      expires_at
                   )", [])?;

  Ok(())
}

#[rocket::main]
async fn main() {
  SimpleLogger::new()
    .with_level(LevelFilter::Info)
    .with_module_level("rocket", LevelFilter::Warn)
    .with_module_level("_", LevelFilter::Warn)
    .init()
    .unwrap();

  let gamestate = Arc::new(Mutex::new(GameState::new()));

  // Internal objects
  let (tx, rx) = mpsc::channel::<GameCommand>();
  let creds = Credentials::from_env().expect("Credentials not in .env-File");
  let db = rusqlite::Connection::open("./backend_db.sqlite3").expect("Could not open database");
  db_init(&db).expect("Error on initialising database");
  let mut spotify_client = AuthCodeSpotify::with_config(creds,
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
                                                   .expect("Credentials not in .env-File"),
                                                    Config { token_cached: true, ..Default::default() });
  match spotify_client.read_token_cache(true) {
    Ok(token) => {
      *spotify_client.get_token().lock().unwrap() = token;
      match spotify_client.refresh_token() {
        Ok(()) => log::info!("Refreshed token"),
        Err(e) => log::warn!("Could not refresh token on start: {:?}", e)
      }
    },
    Err(e) => log::warn!("Could not load token: {:?}", e)
  }
  let references = Arc::new(Mutex::new(
    GameReferences { tx_commands: tx, spotify_client, db }));

  let mut game_pref = GamePreferences::new();
  if let Ok(file) = fs::File::open(PREFERENCES_FILE) {
    if let Ok(p) = serde_json::from_reader::<fs::File, GamePreferences>(file) {
      game_pref = p;
    }
  }
  let preferences = Arc::new(Mutex::new(game_pref));

  // Spawn Game thread
  let g = gamestate.clone();
  let p = preferences.clone();
  let r = references.clone();
  let handle_gamethread = thread::spawn(move || { game::run(g, rx, p, r) });

  // Spawn spotify thread
  let g = gamestate.clone();
  let p = preferences.clone();
  let r = references.clone();
  let handle_spotifythread = thread::spawn(move || { spotify_loop(g, p, r) });

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

  handle_gamethread.join().unwrap();
  handle_spotifythread.join().unwrap();
  // thread join
  log::info!("ende");
}

// für api abfragen (spotify) reqwest
