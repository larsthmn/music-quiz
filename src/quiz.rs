use std::cmp::max;
use std::io::Cursor;
use crate::game::{Question, AnswerExposed};
use rand::distributions::{Standard, Distribution};
use rand::{random, Rng, thread_rng};
use rand::prelude::IteratorRandom;
use rand::seq::SliceRandom;
use rspotify::{AuthCodeSpotify};
use rspotify::clients::{BaseClient};
use rspotify::model::{FullTrack, Id, IdError, PlayableItem, PlaylistId};
use crate::spotify::CustomSpotifyChecks;

// todo: make it configurable
// punkteanzeige nach tippen
// Modi: Keine Anzeige der ausgewählten Antworten
// keine doppelten Lieder
// vorherige Einstellungen der Runde speichern
// Anzahl Runden im Startfenster einstellbar machen
// auflösung mit artist und titel
const ANSWER_COUNT: u32 = 4;

#[derive(Debug, Copy, Clone)]
enum AskedElement {
  Title,
  Artist,
}

impl Distribution<AskedElement> for Standard {
  fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> AskedElement {
    match rng.gen_range(0..=1) {
      0 => AskedElement::Title,
      _ => AskedElement::Artist
    }
  }
}

pub struct SongQuestion {
  songid: String,
  preview_mp3: Option<bytes::Bytes>,
  artist: String,
  title: String,
  asked: AskedElement,
}

pub struct SongQuiz<'a> {
  // additional information about the questions/songs
  songs: Vec<SongQuestion>,
  // questions exposed for Quiz trait
  questions: Vec<Question>,

  spotify: &'a AuthCodeSpotify,

  stream: rodio::OutputStream,
  stream_handle: rodio::OutputStreamHandle,
  sink: Option<rodio::Sink>,
}

impl<'a> SongQuiz<'a> {
  pub fn new(auth: &'a AuthCodeSpotify) -> SongQuiz {
    let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    SongQuiz { songs: vec![], questions: vec![], spotify: auth, stream, stream_handle, sink: None}
  }
}

// impl<'a> Quiz for SongQuiz<'a> {
impl<'a> SongQuiz<'a> {
  /// Generates questions from the selected playlist saved internally
  pub fn generate_questions(&mut self, count: u32, playlist_id: &String) -> Result<(), QuizError> {
    let mut songs: Vec<SongQuestion> = vec![];
    let mut questions: Vec<Question> = vec![];

    if !self.spotify.has_token() {
      return Err(QuizError::SpotifyAPIError("No spotify token"));
    }

    let p_id = PlaylistId::from_uri(playlist_id.as_str())?;
    // Limiting fields like Some("limit,next,offset,total,href,items(is_local,track)") is not possible without
    // including all fields in PlayableItem needed to deserialize it (so I don't)
    let tracks: Vec<rspotify::model::FullTrack> = self.spotify
      .playlist_items(&p_id, None, None)
      .filter_map(|res| res.ok())
      .filter_map(|item| item.track)
      .filter_map(|item| match item {
        PlayableItem::Track(t) => Some(t),
        PlayableItem::Episode(_) => None
      })
      .filter(|item| item.preview_url.is_some())
      .collect();

    // Vectors needed for deduplication to not have the same answer twice
    let mut songnames: Vec<String> = tracks
      .iter()
      .map(|track| track.name.clone())
      .collect();
    songnames.sort();
    songnames.dedup();
    let mut artists: Vec<String> = tracks
      .iter()
      .map(|track| track.artists.first().unwrap().name.clone())
      .collect();
    artists.sort();
    artists.dedup();

    if tracks.len() as u32 <= max(count, ANSWER_COUNT) {
      return Err(QuizError::RuntimeError("Playlist does not have enough songs"));
    }

    // Choose songs to guess first to not have them twice
    let correct_songs: Vec<FullTrack> = tracks
      .choose_multiple(&mut thread_rng(), count as usize)
      .cloned()
      .collect();

    for i in 0..count {
      // Choose song from playlist as correct answer
      let asked: AskedElement = random();
      let correct_song = &correct_songs[i as usize];
      songs.push(SongQuestion {
        songid: correct_song.id.as_ref().ok_or(QuizError::RuntimeError("Song has no song ID"))?.to_string(),
        artist: correct_song.artists.first().ok_or(QuizError::RuntimeError("Song has no artist"))?.name.clone(), // todo: join all artists
        title: correct_song.name.clone(),
        preview_mp3: match &correct_song.preview_url {
          Some(url) => {
            let resp = reqwest::blocking::get(url).expect("No preview gotten");
            Some(resp.bytes().unwrap())
          },
          None => None
        },
        asked,
      });

      // todo: do not take string as id

      let mut answers: Vec<AnswerExposed> = match asked {
        AskedElement::Title => {
          songnames
            .iter()
            .filter(|name| *name != &correct_song.name)
            .choose_multiple(&mut thread_rng(), (ANSWER_COUNT - 1) as usize)
            .iter()
            .map(|song| AnswerExposed { text: (*song).clone(), id: (*song).clone() })
            .collect()
        }
        AskedElement::Artist => {
          artists
            .iter()
            .filter(|artist| *artist != &correct_song.artists.first().unwrap().name)
            .choose_multiple(&mut thread_rng(), (ANSWER_COUNT - 1) as usize)
            .iter()
            .map(|artist| AnswerExposed { text: (*artist).clone(), id: (*artist).clone() })
            .collect()
        }
      };
      let correct_string = match asked {
        AskedElement::Title => correct_song.name.clone(),
        AskedElement::Artist => correct_song.artists.first().unwrap().name.clone()
      };
      let correct_answer = AnswerExposed { text: correct_string.clone(), id: correct_string.clone() };
      answers.push(correct_answer.clone());
      answers.shuffle(&mut thread_rng());

      questions.push(Question {
        text: match asked {
          AskedElement::Title => "Wie heißt der Titel?".to_string(),
          AskedElement::Artist => "Wie heißt der Künstler?".to_string()
        },
        answers,
        correct: Some(correct_answer.id),
        index: i as i32,
        total_questions: count,
      })
    }
    self.songs = songs;
    self.questions = questions;

    Ok(())
  }

  pub fn begin_question_action(&mut self, index: usize) -> Result<(), &'static str> {
    if index > self.songs.len() {
      Err("Invalid song index")
    } else {
      if let Some(bytes) = self.songs[index].preview_mp3.take() {
        // see https://github.com/RustAudio/rodio/issues/171, sink cannot be stopped and play sounds afterwards
        // so we have to create a new one every time
        self.sink = Some(rodio::Sink::try_new(&self.stream_handle).unwrap());
        let cursor = Cursor::new(bytes);
        let source = rodio::Decoder::new(cursor).unwrap();
        self.sink.as_ref().unwrap().append(source);
      } else
      {
        log::warn!("no preview for song {}, fall back to spotify player", self.songs[index].title);
      }

      log::info!("Begin question {} {} - {} with token {:?}", index, self.songs[index].artist, self.songs[index].title, self.spotify);
      Ok(())
    }
  }

  pub fn stop_question_action(&mut self, index: usize) -> Result<(), &'static str> {
    if index > self.songs.len() {
      Err("Invalid song index")
    } else {
      self.sink = None;
      log::info!("End question {} {} - {}", index, self.songs[index].artist, self.songs[index].title);
      Ok(())
    }
  }

  pub fn get_questions(&self) -> &Vec<Question> {
    &self.questions
  }
}

// pub trait Quiz {
//   fn generate_questions(&mut self, count: u32, playlist_id: PlaylistId);
//   fn begin_question_action(&self, index: usize) -> Result<(), &'static str>;
//   fn stop_question_action(&self, index: usize) -> Result<(), &'static str>;
//   fn get_questions(&self) -> &Vec<Question>;
// }


#[derive(Debug, thiserror::Error)]
pub enum QuizError {
  #[error("Error when using spotify API: {0}")]
  SpotifyAPIError(&'static str),

  #[error("RSpotifyError: {0}")]
  RSpotifyIdError(#[from] IdError),

  #[error("RuntimeError: {0}")]
  RuntimeError(&'static str),
}