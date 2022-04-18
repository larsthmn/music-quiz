use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rocket::serde::{Deserialize, Serialize};
use rspotify::AuthCodeSpotify;
use crate::game::GameError::{AnswerNotAllowed, InvalidState};
use crate::quiz::{Quiz, SongQuiz};
use crate::Ready;

#[derive(Serialize, Clone)]
pub struct UserAnswerExposed {
  answer_id: u32,
  user: String,
  ts: u64,
}

#[derive(Serialize, Clone)]
pub struct AnswerExposed {
  pub text: String,
  pub id: u32,
}

#[derive(Serialize, Clone)]
pub struct Question {
  pub text: String,
  pub answers: Vec<AnswerExposed>,
  pub correct: u32,
  pub index: i32,
  pub total_questions: u32,
}

#[derive(Serialize, Clone)]
struct PlayerScoreAPI {
  player: String,
  points: i32,
  correct: u32,
  answers_given: u32,
}

impl PlayerScoreAPI {
  pub fn new(player: String) -> PlayerScoreAPI {
    PlayerScoreAPI {
      player,
      points: 0,
      correct: 0,
      answers_given: 0,
    }
  }
}

#[derive(PartialEq, Serialize, Copy, Clone, Debug, strum_macros::Display)]
pub enum AppStatus {
  Shutdown,
  Ready,
  BeforeGame,
  InGameAnswerPending,
  InGameWaitForNextQuestion,
  BetweenRounds,
}

// Public game management structure
#[derive(Serialize, Clone)]
pub struct GameState {
  status: AppStatus,
  action_start: u64,
  next_action: u64,
  current_question: Option<Question>,
  players: Vec<PlayerScoreAPI>,
  given_answers: Vec<UserAnswerExposed>,
}

// Internal game management structure
pub struct GameReferences {
  pub tx_commands: mpsc::Sender<GameCommand>,
  pub spotify_client: AuthCodeSpotify
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, FromFormField)]
pub enum ScoreMode {
  Time,
  WrongFalse,
  Order,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GamePreferences {
  pub scoremode: ScoreMode,
  pub playlists: Vec<String>,
  pub selected_playlist: String,
  pub time_to_answer: u32,
  pub time_between_answers: u32,
  pub time_before_round: u32,
}

impl GamePreferences {
  pub fn new() -> GamePreferences {
    GamePreferences {
      scoremode: ScoreMode::WrongFalse,
      playlists: vec!["P1".to_string(), "P2".to_string()], // todo: Change back to vec![]
      selected_playlist: "".to_string(),
      time_to_answer: 5,
      time_before_round: 3,
      time_between_answers: 5
    }
  }
}

#[derive(PartialEq, Serialize, Clone, Debug, strum_macros::Display)]
pub enum GameCommand {
  StartGame,
  StopGame,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnswerFromUser {
  id: u32,
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
      println!("Answer with ID {}", answer.id);
      let selected_answer = current_question.answers
        .iter_mut()
        .find(|a| a.id == answer.id);
      if let Some(ans) = selected_answer {
        println!("User {} selected {} at {}", answer.user, ans.text, answer.timestamp);
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
fn game_round(state: &Arc<Mutex<GameState>>, rx: &Receiver<GameCommand>, pref: GamePreferences, spotify: AuthCodeSpotify) {
  let mut s = state.lock().unwrap();
  let next_timeout = prepare_round(&mut s, &pref);
  drop(s);

  // Generate questions to be answered
  let mut quiz = SongQuiz::new(&spotify);
  quiz.generate_questions(4);

  // Wait for game start or stopping game
  if !wait_for_command(&rx, GameCommand::StopGame, next_timeout) {
    // Init results of this round
    for question in quiz.get_questions() {
      quiz.begin_question_action(question.index as usize);

      // Set new question
      let mut s = state.lock().unwrap();
      let next_timeout = set_question(question.clone(), &mut s, &pref);
      drop(s);

      // Wait for users to answer or stopping game
      if wait_for_command(&rx, GameCommand::StopGame, next_timeout) {
        break;
      }

      quiz.stop_question_action(question.index as usize);

      // Evaluate answers
      let mut s = state.lock().unwrap();
      let next_timeout = finish_question(&question, &mut s, &pref);
      drop(s);

      // Wait for next question or stopping game
      if wait_for_command(&rx, GameCommand::StopGame, next_timeout) {
        break;
      }
    }
  }

  // show results
  let mut s = state.lock().unwrap();
  end_round(&mut s);
  drop(s);
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
  s.current_question = None;
  s.action_start = 0;
  s.next_action = 0;
  s.status = AppStatus::BetweenRounds;
}

/// Evaluate answers of users and set game state accordingly
fn finish_question(question: &Question, s: &mut GameState, pref: &GamePreferences) -> u64 {
  println!("Question no {} / {} finished!", question.index + 1, question.total_questions);
  s.status = AppStatus::InGameWaitForNextQuestion;
  if let Some(q) = &mut s.current_question {
    // Publish correct index
    q.correct = question.correct;
  }
  calc_points(s, pref);
  s.players.sort_by(|a, b| b.points.cmp(&a.points));
  let now = s.next_action;
  s.action_start = now;
  s.next_action = now + (pref.time_between_answers * 1000) as u64;
  let next_timeout = s.next_action;
  next_timeout
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
      let points_if_correct = match pref.scoremode {
        ScoreMode::Time => ((1.0 - (user_ans.ts - s.action_start) as f32 /
          (s.next_action - s.action_start) as f32) * 100.0).round() as i32,
        ScoreMode::Order => (100 - pos * 10) as i32,
        ScoreMode::WrongFalse => 100
      };
      let score = s.players
        .iter_mut()
        .find(|score| score.player == user_ans.user)
        .expect("Player must be in Vector");
      score.answers_given += 1;
      if user_ans.answer_id == q.correct {
        score.correct += 1;
        score.points += points_if_correct;
      }
    }
  }
}

/// Set the current question to be answered
fn set_question(mut question: Question, s: &mut GameState, pref: &GamePreferences) -> u64 {
  println!("Question no {} / {}", question.index + 1, question.total_questions);
  question.correct = 0;
  s.current_question = Some(question);
  let now = s.next_action;
  s.action_start = now;
  s.next_action = now + (pref.time_to_answer * 1000) as u64;
  s.status = AppStatus::InGameAnswerPending;
  s.given_answers = vec![];
  s.next_action
}

/// Wait for a command or until some time in ms after epoch
fn wait_for_command(rx: &Receiver<GameCommand>, command: GameCommand, until: u64) -> bool {
  loop {
    let diff: i64 = (until - get_epoch_ms()) as i64;
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
pub fn run(state: Arc<Mutex<GameState>>, rx: mpsc::Receiver<GameCommand>, preferences: Arc<Mutex<GamePreferences>>,
           references: Arc<Mutex<GameReferences>>) {

  // Wait for start by admin?
  let mut s = state.lock().unwrap();
  s.status = Ready;
  s.players = vec![];
  s.action_start = 0;
  s.next_action = 0;
  s.current_question = None;
  drop(s);

  loop {
    // wait for game start
    println!("Start round");
    wait_for_game_start(&rx);

    let p_mut = preferences.lock().unwrap();
    // todo: validate validity of spotify auth and maybe refresh
    let pref = p_mut.clone();
    drop(p_mut);
    let r_mut = references.lock().unwrap();
    let spotify = r_mut.spotify_client.clone();
    drop(r_mut);
    game_round(&state, &rx, pref, spotify);
    // After the round the results are available to be fetched until the next round is started

    println!("Round ended");
  }
}

/// Wait for the Command `StartGame`
fn wait_for_game_start(rx: &Receiver<GameCommand>) {
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
}