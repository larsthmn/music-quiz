#[macro_use]
extern crate rocket;

use std::sync::{Arc, mpsc, Mutex};
use std::{fs, thread};
use rocket::fs::FileServer;
use rocket::serde::{json::Json, Serialize, Deserialize};
use rocket::State;
use crate::game::{AnswerFromUser, GameState, GameReferences, GameCommand, GamePreferences, ScoreMode};
use std::time::{SystemTime, UNIX_EPOCH};
use rspotify::{AuthCodeSpotify, Config, Credentials, OAuth};
use rspotify::clients::{BaseClient, OAuthClient};
use simple_logger::SimpleLogger;
use log::{LevelFilter};
use rocket::serde::json::serde_json;
use crate::spotify::{spotify_loop};
use ts_rs::TS;

mod game;
mod quiz;
mod spotify;

// todo: switch from rocket to axum, has websockets included

const PREFERENCES_FILE: &'static str = "preferences.json";
const SPOTIFY_FILE: &'static str = "spotify.json";

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

#[post("/refresh_spotify")]
fn refresh_spotify(references: &State<Arc<Mutex<GameReferences>>>) {
  let r = references.lock().unwrap();
  if let Err(e) =  r.tx_spotify.send(()) {
    log::warn!("Could not send spotify wakeup ({:?})", e)
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

#[post("/set?<scoremode>&<playlist>&<time_to_answer>&<time_between_answers>&<time_before_round>&<rounds>&<preview_mode>&<hide_answers>&<ask_for_artist>&<ask_for_title>")]
fn set_preference(preferences: &State<Arc<Mutex<GamePreferences>>>,
                  scoremode: Option<ScoreMode>,
                  playlist: Option<String>,
                  time_to_answer: Option<u32>,
                  time_between_answers: Option<u32>,
                  time_before_round: Option<u32>,
                  rounds: Option<u32>,
                  preview_mode: Option<bool>,
                  hide_answers: Option<bool>,
                  ask_for_artist: Option<bool>,
                  ask_for_title: Option<bool>)
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
  if let Some(a) = ask_for_title {
    p.ask_for_title = if p.ask_for_artist {a} else {true};
    log::info!("set ask_for_title to {}", p.ask_for_title);
  }
  if let Some(a) = ask_for_artist {
    p.ask_for_artist = if p.ask_for_title {a} else {true};
    log::info!("set ask_for_title to {}", p.ask_for_artist);
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

#[derive(Deserialize, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
struct SpotifyPrefs {
  scopes: Vec<String>,
  redirect_uri: String,
  client_id: String,
  client_secret: String,
}

impl SpotifyPrefs {
  fn new() -> SpotifyPrefs {
    SpotifyPrefs {
      scopes: vec!["user-modify-playback-state".to_string(),
                    "user-read-playback-state".to_string(),
                    "user-read-currently-playing".to_string(),
                    "playlist-read-collaborative".to_string(),
                    "playlist-read-private".to_string(),
                    "app-remote-control".to_string(),
                    "streaming".to_string(),
                    "user-read-email".to_string(),
                    "user-read-private".to_string()],
      redirect_uri: "http://localhost:80/redirect".to_string(),
      client_id: "d071021f312148b38eaa0243f11a52c8".to_string(),
      client_secret: "123456789".to_string()
    }
  }
}

#[get("/<file..>", rank = 2)]
/**
Default route for everything that is not part of the API. URL is parsed and handled by React.
*/
async fn index(file: std::path::PathBuf) -> Option<rocket::fs::NamedFile> {
  println!("match file with {:?}", file);
  rocket::fs::NamedFile::open(std::path::Path::new("public/index.html")).await.ok()
}

#[rocket::main]
async fn main() {
  SimpleLogger::new()
    .with_level(LevelFilter::Info)
    .with_module_level("rocket", LevelFilter::Warn)
    .with_module_level("_", LevelFilter::Warn)
    .init()
    .unwrap();

  // Internal objects
  let (tx_cmd, rx_cmd) = mpsc::channel::<GameCommand>();
  let (tx_spotify, rx_spotify) = mpsc::channel::<()>(); // Nothing to be transferred, just to wakeup

  // Read spotify preferences and create clients
  let mut spotify_prefs = SpotifyPrefs::new();
  if let Ok(file) = fs::File::open(SPOTIFY_FILE) {
    if let Ok(s) = serde_json::from_reader::<fs::File, SpotifyPrefs>(file) {
      spotify_prefs = s;
      log::info!("Successfully read file {}", SPOTIFY_FILE);
    } else {
      log::warn!("File {} not in expected format, using default", SPOTIFY_FILE);
    }
  } else {
    log::warn!("Did not find {}, using default", SPOTIFY_FILE);
  }
  let creds = Credentials {id: spotify_prefs.client_id, secret: Some(spotify_prefs.client_secret) };
  let redirect_uri = spotify_prefs.redirect_uri;
  let mut spotify_client = AuthCodeSpotify::with_config(creds,
                                                        OAuth {
                                                          scopes: spotify_prefs.scopes.into_iter().collect(),
                                                          redirect_uri,
                                                          ..Default::default()
                                                        },
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

  // Shared objects
  let references = Arc::new(Mutex::new(
    GameReferences { tx_commands: tx_cmd, tx_spotify, spotify_client }));
  let mut game_pref = GamePreferences::new();
  if let Ok(file) = fs::File::open(PREFERENCES_FILE) {
    if let Ok(p) = serde_json::from_reader::<fs::File, GamePreferences>(file) {
      game_pref = p;
    }
  }
  let preferences = Arc::new(Mutex::new(game_pref));
  let gamestate = Arc::new(Mutex::new(GameState::new()));

  // Spawn Game thread
  let g = gamestate.clone();
  let p = preferences.clone();
  let r = references.clone();
  let handle_gamethread = thread::spawn(move || { game::run(g, rx_cmd, p, r) });

  // Spawn spotify thread
  let g = gamestate.clone();
  let p = preferences.clone();
  let r = references.clone();
  let handle_spotifythread = thread::spawn(move || { spotify_loop(g, rx_spotify, p, r) });

  // Start HTTP interface
  let fut_rocket = rocket::build()
    .mount("/", routes![select_answer, index,
            get_state, get_time, start_game, stop_game, set_preference, get_preferences, set_preferences,
            authorize_spotify, refresh_spotify])
    .mount("/", FileServer::from("public").rank(1))
    .manage(gamestate)
    .manage(references)
    .manage(preferences)
    .launch();

  // todo: Start websocket server and join futures
  fut_rocket.await.unwrap();

  handle_gamethread.join().unwrap();
  handle_spotifythread.join().unwrap();

  log::info!("Goodbye.");
}