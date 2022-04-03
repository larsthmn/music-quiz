use std::error::Error;
use std::sync::{Arc, Mutex};
use std::{fmt, thread};
use std::fmt::{Display, Formatter};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rocket::serde::{Deserialize, Serialize};
use crate::AppStatus::{InGameAnswerPending, InGameWaitForNextQuestion};
use crate::game::GameError::{AnswerNotAllowed, InvalidState};
use crate::Ready;

#[derive(Serialize, Clone)]
pub struct Answer {
    pub text: String,
    pub id: u64,
    pub selected_by: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct Question {
    pub text: String,
    pub answers: Vec<Answer>,
    pub correct: i32
}

#[derive(PartialEq, Serialize, Copy, Clone, Debug, strum_macros::Display)]
pub enum AppStatus {
    Shutdown,
    Ready,
    InGameAnswerPending,
    InGameWaitForNextQuestion
}

// Main game management structure
#[derive(Serialize, Clone)]
pub struct GameState {
    pub status: AppStatus,
    pub action_start: u64,
    pub next_action: u64,
    pub index: i32,
    pub total_questions: i32,
    pub current_question: Option<Question>
}

#[derive(Serialize, Deserialize)]
pub struct AnswerFromUser {
    id: u64,
    timestamp: u64,
    user: String
}

impl GameState {
    pub fn new() -> GameState {
        GameState {
            status: AppStatus::Shutdown,
            next_action: 0,
            index: 0,
            current_question: None,
            action_start: 0,
            total_questions: 0,
        }
    }

    /// Receive an answer from a user
    pub fn give_answer(&mut self, answer: AnswerFromUser) -> Result<(), GameError> {
        if self.status != AppStatus::InGameAnswerPending {
            return Err(InvalidState(self.status))
        }
        // Check if answers already contain user somewhere
        if let Some(current_question) = &mut self.current_question {
            let user_has_selected = current_question.answers
                .iter()
                .any(|a| a.selected_by.iter().any(|u| *u == answer.user));

            if user_has_selected {
                return Err(AnswerNotAllowed("Already selected an answer"));
            }

            if answer.timestamp < self.action_start || answer.timestamp > self.next_action {
                return Err(AnswerNotAllowed("Timestamp of answer not in allowed range"));
            }

            // Select answer with given ID
            let selected_answer = current_question.answers
                .iter_mut()
                .find(|a| a.id == answer.id);
            if let Some(ans) = selected_answer {
                ans.selected_by.push(answer.user.clone());
                println!("User {} selected {} at {}", answer.user, ans.text, answer.timestamp);
            } else {
                return Err(AnswerNotAllowed("Invalid ID"));
            }

        } else {
            return Err(AnswerNotAllowed("No current question"));
        }
        Ok(())
    }
}

fn get_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is < UNIX_EPOCH")
        .as_millis() as u64
}

fn game_round(state: &Arc<Mutex<GameState>>, questions: Vec<Question>) { // 2nd param trait with iterator over questions and actions like play?

    // todo: as param, configure by admin
    const TIME_TO_ANSWER: u64 = 10000;
    const TIME_BETWEEN_ANSWERS: u64 = 10000;

    let mut s = state.lock().unwrap();
    s.status = Ready;
    s.index = -1;
    s.total_questions = questions.len() as i32;

    drop(s);

    thread::sleep(Duration::from_secs(3));
    for question in questions {
        let mut s = state.lock().unwrap();
        s.current_question = Some(question.clone());
        if let Some(q)  = &mut s.current_question {
            q.correct = -1;
        }
        let now = get_epoch_ms();
        s.action_start = now;
        s.next_action = now + TIME_TO_ANSWER;
        s.index += 1;
        s.status = InGameAnswerPending;
        println!("Question no {} / {}", s.index + 1, s.total_questions);
        drop(s);

        thread::sleep(Duration::from_millis(TIME_TO_ANSWER));

        let mut s = state.lock().unwrap();
        println!("Question no {} / {} finished!", s.index + 1, s.total_questions);
        s.status = InGameWaitForNextQuestion;
        if let Some(q)  = &mut s.current_question {
            q.correct = question.correct;
        }
        let now = get_epoch_ms();
        s.action_start = now;
        s.next_action = now + TIME_BETWEEN_ANSWERS;
        // todo: Evaluate answers
        drop(s);

        thread::sleep(Duration::from_millis(TIME_BETWEEN_ANSWERS));
    }

    // show results
    let mut s = state.lock().unwrap();
    s.current_question = None;
    let now = get_epoch_ms();
    s.action_start = now;
    s.next_action = now + TIME_BETWEEN_ANSWERS;
    s.index = -1;
    s.status = Ready;
    drop(s);

}

pub fn run(state: Arc<Mutex<GameState>>) {
    loop {
        // Wait for start by admin?
        println!("Wait for start by admin");
        thread::sleep(Duration::from_secs(3));

        // game start
        println!("Start round");
        let questions = vec![
            Question {
                text: "Frage 1".to_string(),
                answers: vec![Answer { text: "A11 richtig".to_string(), id: 11, selected_by: vec![] },
                              Answer { text: "A12 falsch".to_string(), id: 12, selected_by: vec![] },
                              Answer { text: "A13 falsch".to_string(), id: 13, selected_by: vec![] },
                              Answer { text: "A14 falsch".to_string(), id: 14, selected_by: vec![] }],
                correct: 11,
            },
            Question {
                text: "Frage 2".to_string(),
                answers: vec![Answer { text: "A21 falsch".to_string(), id: 21, selected_by: vec![] },
                              Answer { text: "A22 richtig".to_string(), id: 22, selected_by: vec![] },
                              Answer { text: "A23 falsch".to_string(), id: 23, selected_by: vec![] },
                              Answer { text: "A24 falsch".to_string(), id: 24, selected_by: vec![] }],
                correct: 22,
            },
            Question {
                text: "Frage 3".to_string(),
                answers: vec![Answer { text: "A31 falsch".to_string(), id: 31, selected_by: vec![] },
                              Answer { text: "A32 falsch".to_string(), id: 32, selected_by: vec![] },
                              Answer { text: "A33 richtig".to_string(), id: 33, selected_by: vec![] },
                              Answer { text: "A34 falsch".to_string(), id: 34, selected_by: vec![] }],
                correct: 33,
            }];
        game_round(&state, questions);
        println!("Round ended");
        // let mut s = state.lock().unwrap();
        // next_question(s.deref_mut());
        // drop(s);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GameError {
    #[error("Answer not allowed: {0}")]
    AnswerNotAllowed(&'static str),

    #[error("Invalid game state {0}")]
    InvalidState(AppStatus),
}