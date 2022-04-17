use crate::game::{Question, AnswerExposed};
use rand::distributions::{Standard, Distribution};
use rand::{random, Rng, thread_rng};
use rand::seq::SliceRandom;
use crate::quiz::AskedElement::{Artist, Title};

const ANSWER_COUNT: u32 = 4;

#[derive(Debug, Copy, Clone)]
enum AskedElement {
  Title,
  Artist,
}

impl Distribution<AskedElement> for Standard {
  fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> AskedElement {
    match rng.gen_range(0..=1) {
      0 => Title,
      _ => Artist
    }
  }
}

pub struct SongQuestion {
  songid: u64,
  artist: String,
  title: String,
  asked: AskedElement,
}

pub struct SongQuiz {
  songs: Vec<SongQuestion>,
  // additional information about the questions/songs
  questions: Vec<Question>,
  // questions exposed for Quiz trait
}

impl SongQuiz {
  pub fn new() -> SongQuiz {
    SongQuiz { songs: vec![], questions: vec![] }
  }
}

impl Quiz for SongQuiz {
  fn generate_questions(&mut self, count: u32) {
    let mut songs: Vec<SongQuestion> = vec![];
    let mut questions: Vec<Question> = vec![];
    for i in 0..count {
      let asked: AskedElement = random();
      songs.push(SongQuestion {
        songid: i as u64,
        artist: format!("artist {}", i),
        title: format!("song {}", i),
        asked,
      });
      let mut answers = vec![];
      for a in 0..(ANSWER_COUNT - 1) {
        answers.push(AnswerExposed { text: format!("Falsch {}", a), id: random() });
      }
      let correct_uuid = random();
      answers.push(AnswerExposed { text: "Richtig".to_string(), id: correct_uuid });
      answers.shuffle(&mut thread_rng());
      questions.push(Question {
        text: match asked {
          Title => "Wie heißt der Titel?".to_string(),
          Artist => "Wie heißt der Künstler?".to_string()
        },
        answers,
        correct: correct_uuid,
        index: i as i32,
        total_questions: count,
      })
    }
    self.songs = songs;
    self.questions = questions;
  }

  fn begin_question_action(&self, index: usize) -> Result<(), ()> {
    if index > self.songs.len() {
      Err(())
    } else {
      println!("Begin question {} {} - {}", index, self.songs[index].artist, self.songs[index].title);
      Ok(())
    }
  }

  fn stop_question_action(&self, index: usize) -> Result<(), ()> {
    if index > self.songs.len() {
      Err(())
    } else {
      println!("End question {} {} - {}", index, self.songs[index].artist, self.songs[index].title);
      Ok(())
    }
  }
}

pub trait Quiz {
  fn generate_questions(&mut self, count: u32);
  fn begin_question_action(&self, index: usize) -> Result<(), ()>;
  fn stop_question_action(&self, index: usize) -> Result<(), ()>;
}

pub struct SongQuizIterator<'a> {
  songquiz: &'a SongQuiz,
  index: usize,
}

impl<'a> IntoIterator for &'a SongQuiz {
  type Item = Question;
  type IntoIter = SongQuizIterator<'a>;

  fn into_iter(self) -> Self::IntoIter {
    SongQuizIterator { songquiz: self, index: 0 }
  }
}

impl<'a> Iterator for SongQuizIterator<'a> {
  type Item = Question;

  fn next(&mut self) -> Option<Self::Item> {
    if self.index < self.songquiz.questions.len() {
      self.index += 1;
      Some(self.songquiz.questions[self.index - 1].clone())
    } else {
      None
    }
  }
}
