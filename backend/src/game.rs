use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use rspotify::AuthCodeSpotify;
use crate::game::GameError::{AnswerNotAllowed, InvalidState};
use crate::quiz::{QuizError, SongQuiz};
use ts_rs::TS;

const MAX_POINTS_CORRECT_ANSWER: u32 = 100; /// Maximum points for correct answer
const MIN_POINTS_CORRECT_ANSWER: u32 = 20;  /// Minimum points for correct answer
const TIME_FULL_POINTS_MS: u32 = 900;  /// Time after question start in which full points are given (in time score mode)

#[derive(Serialize, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct UserAnswerExposed {
  answer_id: String,
  user: String,
  ts: u64,
}

#[derive(Serialize, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct AnswerExposed {
  pub text: String,
  pub id: String,
}

#[derive(Serialize, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct Question {
  pub text: String,
  pub answers: Vec<AnswerExposed>,
  pub correct: Option<String>,
  pub solution: Option<String>,
  pub index: i32,
  pub total_questions: u32,
}

#[derive(Serialize, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
struct PlayerScoreAPI {
  player: String,
  points: i32,
  correct: u32,
  answers_given: u32,
  last_points: Option<i32>,
  last_time: Option<f32>
}

impl PlayerScoreAPI {
  pub fn new(player: String) -> PlayerScoreAPI {
    PlayerScoreAPI {
      player,
      points: 0,
      correct: 0,
      answers_given: 0,
      last_points: None,
      last_time: None,
    }
  }
}

#[derive(PartialEq, Serialize, Copy, Clone, Debug, strum_macros::Display, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub enum AppStatus {
  Shutdown,
  Ready,
  BeforeGame,
  InGameAnswerPending,
  InGameWaitForNextQuestion,
  BetweenRounds,
}

// Public game management structure
#[derive(Serialize, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct GameState {
  status: AppStatus,
  action_start: u64,
  next_action: u64,
  current_question: Option<Question>,
  players: Vec<PlayerScoreAPI>,
  given_answers: Vec<UserAnswerExposed>,
  hide_answers: bool
}

// #[derive(Serialize, Clone, TS)]
// #[ts(export)]
// #[ts(export_to = "../shared/")]
// pub struct GameStateExposed {
//   status: AppStatus,
//   action_start: u64,
//   next_action: u64,
//   current_question: Option<Question>,
//   players: Vec<PlayerScoreAPI>,
//   given_answers: Vec<UserAnswerExposed>,
//   hide_answers: bool
// }
//
// impl From<GameState> for GameStateExposed {
//   fn from(g: GameState) -> Self {
//     GameStateExposed {
//       status: g.status,
//       action_start: g.action_start,
//       next_action: g.next_action,
//       current_question: g.current_question,
//       players: g.players,
//       given_answers: g.given_answers,
//       hide_answers: g.hide_answers
//     }
//   }
// }

// Internal game management structure
pub struct GameReferences {
  pub tx_commands: mpsc::Sender<GameCommand>,
  pub tx_spotify: mpsc::Sender<()>,
  pub spotify_client: AuthCodeSpotify,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub enum ScoreMode {
  Time,
  WrongFalse,
  Order,
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct Playlist {
  pub name: String,
  pub id: String,
}

#[derive(Serialize, Deserialize, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct GamePreferences {
  pub scoremode: ScoreMode,
  pub playlists: Vec<Playlist>,
  pub selected_playlist: Option<Playlist>,
  pub time_to_answer: u32,
  pub time_between_answers: u32,
  pub time_before_round: u32,
  pub rounds: u32,
  pub preview_mode: bool,
  pub hide_answers: bool,
  pub ask_for_artist: bool,
  pub ask_for_title: bool
}

impl GamePreferences {
  pub fn new() -> GamePreferences {
    GamePreferences {
      scoremode: ScoreMode::WrongFalse,
      playlists: vec![],
      selected_playlist: None,
      time_to_answer: 5,
      time_before_round: 3,
      time_between_answers: 5,
      rounds: 5,
      preview_mode: false,
      hide_answers: false,
      ask_for_artist: true,
      ask_for_title: true
    }
  }
}

#[derive(PartialEq, Serialize, Clone, Debug, strum_macros::Display, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub enum GameCommand {
  StartGame,
  StopGame,
}

#[derive(Serialize, Deserialize, Debug, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct AnswerFromUser {
  id: String,
  timestamp: u64,
  user: String,
}

impl GameState {
  pub fn new() -> GameState {
    GameState {
      status: AppStatus::Shutdown,
      action_start: 0,
      next_action: 0,
      current_question: None,
      players: vec![],
      given_answers: vec![],
      hide_answers: false
    }
  }

  /// Receive an answer from a user
  pub fn give_answer(&mut self, answer: AnswerFromUser) -> Result<(), GameError> {
    if self.status != AppStatus::InGameAnswerPending {
      return Err(InvalidState(self.status));
    }
    // Check if answers already contain user somewhere
    if let Some(current_question) = &mut self.current_question {
      let user_has_selected = self.given_answers
        .iter()
        .any(|a| a.user == answer.user);

      if user_has_selected {
        return Err(AnswerNotAllowed("Already selected an answer"));
      }

      if answer.timestamp < self.action_start || answer.timestamp > self.next_action {
        return Err(AnswerNotAllowed("Timestamp of answer not in allowed range"));
      }

      // Select answer with given ID
      log::info!("Answer with ID {}", answer.id);
      let selected_answer = current_question.answers
        .iter_mut()
        .find(|a| a.id == answer.id);
      if let Some(ans) = selected_answer {
        log::info!("User {} selected {} at {}", answer.user, ans.text, answer.timestamp);
        self.given_answers.push(
          UserAnswerExposed { user: answer.user.clone(), ts: answer.timestamp, answer_id: answer.id });
      } else {
        return Err(AnswerNotAllowed("Invalid ID"));
      }
    } else {
      return Err(AnswerNotAllowed("No current question"));
    }
    Ok(())
  }
}

/// Return milliseconds from epoch.
fn get_epoch_ms() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("System time is < UNIX_EPOCH")
    .as_millis() as u64
}

/// Executes a game round:
///
/// Init => for each `question` [set question => wait for answer] => show results.
/// Preferences stay the same for the whole round.
fn game_round(state: &Arc<RwLock<GameState>>, rx: &mpsc::Receiver<GameCommand>, pref: GamePreferences, spotify: AuthCodeSpotify) -> Result<(), GameError> {
  // todo: waiting state while questions are prepared?
  // Generate questions to be answered
  let mut quiz = SongQuiz::new(&spotify, pref.preview_mode);
  quiz.generate_questions(pref.rounds,
                          &pref.selected_playlist.as_ref().ok_or(GameError::RuntimeError("No playlist selected"))?.id,
                          pref.ask_for_artist,
                          pref.ask_for_title)?;

  let mut s = state.write().unwrap();
  let next_timeout = prepare_round(&mut s, &pref);
  drop(s);

  // Wait for game start or stopping game
  if !wait_for_command(&rx, GameCommand::StopGame, next_timeout) {
    // Init results of this round
    for question in quiz.get_questions().clone() {
      if let Err(e) = quiz.begin_question_action(question.index as usize) {
        log::warn!("Begin question failed with error: {:?}", e);
      }

      // Set new question
      let mut s = state.write().unwrap();
      let next_timeout = set_question(question.clone(), &mut s, &pref);
      drop(s);

      // Wait for users to answer or stopping game
      if wait_for_command(&rx, GameCommand::StopGame, next_timeout) {
        break;
      }

      if let Err(e) = quiz.stop_question_action(question.index as usize){
        log::warn!("End question failed with error: {:?}", e);
      }


      // Evaluate answers
      let mut s = state.write().unwrap();
      let next_timeout = finish_question(&question, &mut s, &pref);
      drop(s);

      // Wait for next question or stopping game
      if wait_for_command(&rx, GameCommand::StopGame, next_timeout) {
        break;
      }
    }
  }

  // show results
  let mut s = state.write().unwrap();
  end_round(&mut s);
  drop(s);

  Ok(())
}

fn prepare_round(s: &mut GameState, pref: &GamePreferences) -> u64 {
  s.players = vec![];
  s.current_question = None;
  s.status = AppStatus::BeforeGame;
  let now = get_epoch_ms();
  s.action_start = now;
  s.next_action = now + (pref.time_before_round * 1000) as u64;
  s.given_answers = vec![];
  let next_timeout = s.next_action;
  next_timeout
}

fn end_round(s: &mut GameState) {
  for score in &mut s.players {
    score.last_time = None;
    score.last_points = None;
  }
  s.current_question = None;
  s.action_start = 0;
  s.next_action = 0;
  s.status = AppStatus::BetweenRounds;
}

/// Evaluate answers of users and set game state accordingly
fn finish_question(question: &Question, s: &mut GameState, pref: &GamePreferences) -> u64 {
  log::info!("Question no {} / {} finished!", question.index + 1, question.total_questions);
  s.status = AppStatus::InGameWaitForNextQuestion;
  if let Some(q) = &mut s.current_question {
    // Publish correct index
    q.correct = question.correct.clone();
    q.solution = question.solution.clone();
  }
  calc_points(s, pref);
  s.players.sort_by(|a, b| b.points.cmp(&a.points));
  let now = s.next_action;
  s.action_start = now;
  s.hide_answers = false;
  s.next_action = now + (pref.time_between_answers * 1000) as u64;
  let next_timeout = s.next_action;
  next_timeout
}

/**
Calculate the points depending on needed time and maximum time to answer
*/
fn calc_points_time(time_needed_for_answer: u64, max_answer_time: u64) -> u32 {
  if time_needed_for_answer <= TIME_FULL_POINTS_MS as u64 {
    MAX_POINTS_CORRECT_ANSWER
  } else {
    let time_after_deadzone = std::cmp::max(time_needed_for_answer - TIME_FULL_POINTS_MS as u64, 0);
    let part_needed = time_after_deadzone as f32 / (max_answer_time - TIME_FULL_POINTS_MS as u64) as f32;
    ((1.0 - part_needed).max(0.0) *
      (MAX_POINTS_CORRECT_ANSWER - MIN_POINTS_CORRECT_ANSWER) as f32 + MIN_POINTS_CORRECT_ANSWER as f32)
      .round() as u32
  }
}

/// Calculate the points for all players for the current question
fn calc_points(s: &mut GameState, pref: &GamePreferences) {
  if let Some(q) = &s.current_question {
    let given_answers = &mut s.given_answers;
    given_answers.sort_by(|a, b| a.ts.cmp(&b.ts));
    for (pos, user_ans) in given_answers.iter().enumerate() {
      // find player in results
      if !s.players.iter_mut().any(|score| score.player == user_ans.user) {
        s.players.push(PlayerScoreAPI::new(user_ans.user.clone()));
      }
      // Points need to be calculated here, because later s can't be borrowed (since score = mutable borrow)
      let time_needed_for_answer = user_ans.ts - s.action_start;
      let mut points_if_correct: i32 = match pref.scoremode {
        ScoreMode::Time => calc_points_time(time_needed_for_answer, s.next_action - s.action_start) as i32,
        ScoreMode::Order => (MAX_POINTS_CORRECT_ANSWER - pos as u32 * 10) as i32,
        ScoreMode::WrongFalse => MAX_POINTS_CORRECT_ANSWER as i32
      };
      points_if_correct = std::cmp::max(MIN_POINTS_CORRECT_ANSWER as i32,
                                        std::cmp::min(MAX_POINTS_CORRECT_ANSWER as i32, points_if_correct));
      let score = s.players
        .iter_mut()
        .find(|score| score.player == user_ans.user)
        .expect("Player must be in Vector");
      score.answers_given += 1;
      score.last_time = Some(time_needed_for_answer as f32 / 1000.0);
      if &user_ans.answer_id == q.correct.as_ref().expect("No correct answer in calc_points") {
        score.correct += 1;
        score.last_points = Some(points_if_correct);
        score.points += points_if_correct;
      } else {
        score.last_points = Some(0);
      }
    }
  }
}

/// Set the current question to be answered
fn set_question(mut question: Question, s: &mut GameState, pref: &GamePreferences) -> u64 {
  log::info!("Question no {} / {}", question.index + 1, question.total_questions);
  question.correct = None;
  question.solution = None;
  s.current_question = Some(question);
  let now = s.next_action;
  s.action_start = now;
  s.next_action = now + (pref.time_to_answer * 1000) as u64;
  s.status = AppStatus::InGameAnswerPending;
  s.given_answers = vec![];
  s.hide_answers = if pref.hide_answers {true} else {false};
  s.next_action
}

/// Wait for a command or until some time in ms after epoch
fn wait_for_command(rx: &mpsc::Receiver<GameCommand>, command: GameCommand, until: u64) -> bool {
  loop {
    let diff: i64 = (until.checked_sub(get_epoch_ms()).unwrap_or(100)) as i64;
    if diff > 0 {
      match rx.recv_timeout(Duration::from_millis((diff) as u64)) {
        Ok(cmd) if cmd == command => {
          return true;
        }
        Ok(_) => {}
        Err(_) => return false
      }
    } else {
      return false;
    }
  }
}

/// Main loop for the game thread. `rx` is used to receive game commands.
pub fn run(state: Arc<RwLock<GameState>>, rx: mpsc::Receiver<GameCommand>, preferences: Arc<Mutex<GamePreferences>>,
           references: Arc<Mutex<GameReferences>>) {

  // Wait for start by admin?
  let mut s = state.write().unwrap();
  s.status = AppStatus::Ready;
  s.players = vec![];
  s.action_start = 0;
  s.next_action = 0;
  s.current_question = None;
  drop(s);

  loop {
    // wait for game start
    log::info!("Start round");
    wait_for_game_start(&rx);

    let p_mut = preferences.lock().unwrap();
    let pref = p_mut.clone();
    drop(p_mut);
    let r_mut = references.lock().unwrap();
    let spotify = r_mut.spotify_client.clone();
    drop(r_mut);
    match game_round(&state, &rx, pref, spotify) {
      Ok(()) => log::info!("Round ended"),
      Err(e) => log::warn!("Round ended with error: {:?}", e)
    }
    // After the round the results are available to be fetched until the next round is started
  }
}

/// Wait for the Command `StartGame`
fn wait_for_game_start(rx: &mpsc::Receiver<GameCommand>) {
  loop {
    if let Ok(c) = rx.recv() {
      if c == GameCommand::StartGame {
        return;
      }
    }
  }
}

#[derive(Debug, thiserror::Error)]
pub enum GameError {
  #[error("Answer not allowed: {0}")]
  AnswerNotAllowed(&'static str),

  #[error("Invalid game state {0}")]
  InvalidState(AppStatus),

  #[error("RuntimeError: {0}")]
  RuntimeError(&'static str),

  #[error("QuizError: {0}")]
  QuizError(QuizError),
}

impl From<&'static str> for GameError {
  fn from(s: &'static str) -> Self {
    GameError::RuntimeError(s)
  }
}

impl From<QuizError> for GameError {
  fn from(e: QuizError) -> Self {
    GameError::QuizError(e)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn points() {
    let points = calc_points_time(1000, 1000);
    assert_eq!(MIN_POINTS_CORRECT_ANSWER, points);
    assert_eq!(MAX_POINTS_CORRECT_ANSWER, calc_points_time(0, 1000));
    assert_eq!((MAX_POINTS_CORRECT_ANSWER + MIN_POINTS_CORRECT_ANSWER) / 2,
               calc_points_time(1000 + TIME_FULL_POINTS_MS as u64,
                                2000 + TIME_FULL_POINTS_MS as u64));
  }
}