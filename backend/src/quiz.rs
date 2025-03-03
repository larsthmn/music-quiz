use std::cmp::max;
use std::sync::Arc;
use crate::game::{Question, AnswerExposed};
use rand::distributions::{Standard, Distribution};
use rand::{random, Rng, thread_rng};
use rand::prelude::IteratorRandom;
use rand::seq::SliceRandom;
use rspotify::{AuthCodeSpotify};
use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::model::{Device, FullTrack, IdError, PlayableItem, PlaylistId};
use rspotify::prelude::PlayableId;
use crate::spotify::CustomSpotifyChecks;
use futures::StreamExt;

// Modi: Keine Anzeige der ausgewählten Antworten
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
  song: FullTrack,
  preview_mp3: Option<bytes::Bytes>,
  _asked: AskedElement, // todo: use or delete
}

pub struct SongQuiz {
  // additional information about the questions/songs
  songs: Vec<SongQuestion>,
  // questions exposed for Quiz trait
  questions: Vec<Question>,

  spotify: Arc<AuthCodeSpotify>,

  // _stream: rodio::OutputStream,
  // stream_handle: rodio::OutputStreamHandle,
  // sink: Option<rodio::Sink>,
  preview_mode: bool,
}

impl SongQuiz {
  pub fn new(auth: Arc<AuthCodeSpotify>, preview_mode: bool) -> SongQuiz {
    // let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    SongQuiz {
      songs: vec![],
      questions: vec![],
      spotify: auth,
      // _stream: stream,
      // stream_handle,
      // sink: None,
      preview_mode,
    }
  }
}

// impl<'a> Quiz for SongQuiz<'a> {
impl SongQuiz {

  /// Generates questions from the selected playlist saved internally
  pub async fn generate_questions(&mut self, count: u32, playlist_id: &String, ask_artists: bool, ask_title: bool) -> Result<(), QuizError> {
    let mut songs: Vec<SongQuestion> = vec![];
    let mut questions: Vec<Question> = vec![];

    if !self.spotify.has_token().await {
      return Err(QuizError::SpotifyAPIError("No spotify token"));
    }

    let p_id = PlaylistId::from_uri(playlist_id.as_str())?;
    let tracks = self.get_tracks(p_id).await;

    // Vectors needed for deduplication to not have the same answer twice
    let songnames = Self::get_songnames(&tracks);
    let artists = Self::get_artists(&tracks);

    if tracks.len() as u32 <= max(count, ANSWER_COUNT) {
      return Err(QuizError::RuntimeError(format!(
        "Playlist has {} tracks, but at least {} are needed",
        tracks.len(),
        max(count, ANSWER_COUNT))));
    }

    // Choose songs to guess first to not have them twice
    let correct_songs: Vec<FullTrack> = tracks
      .choose_multiple(&mut thread_rng(), count as usize)
      .cloned()
      .collect();

    for i in 0..count {
      // Choose song from playlist as correct answer
      let mut asked: AskedElement = random();
      if !ask_artists && ask_title {
        asked = AskedElement::Title;
      } else if ask_artists && !ask_title {
        asked = AskedElement::Artist;
      }

      let correct_song = &correct_songs[i as usize];
      songs.push(SongQuestion {
        song: correct_song.clone(),
        preview_mp3: match &correct_song.preview_url {
          Some(url) => {
            if self.preview_mode {
              let resp = reqwest::blocking::get(url).expect("No preview gotten");
              Some(resp.bytes().unwrap())
            } else {
              None
            }
          }
          None => None
        },
        _asked: asked,
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
        solution: Some(format!("{} - {}", correct_song.artists.first().unwrap().name.clone(), correct_song.name.clone())),
        index: i as i32,
        total_questions: count,
      })
    }
    self.songs = songs;
    self.questions = questions;

    Ok(())
  }

  fn get_artists(tracks: &Vec<FullTrack>) -> Vec<String> {
    let mut artists: Vec<String> = tracks
      .iter()
      .map(|track| track.artists.first().unwrap().name.clone())
      .collect();
    artists.sort();
    artists.dedup();
    artists
  }

  fn get_songnames(tracks: &Vec<FullTrack>) -> Vec<String> {
    let mut songnames: Vec<String> = tracks
      .iter()
      .map(|track| track.name.clone())
      .collect();
    songnames.sort();
    songnames.dedup();
    songnames
  }

  async fn get_tracks(&mut self, p_id: PlaylistId<'_>) -> Vec<FullTrack> {
    let preview_mode = self.preview_mode;

    // Limiting fields like Some("limit,next,offset,total,href,items(is_local,track)") is not possible without
    // including all fields in PlayableItem needed to deserialize it (so I don't)
    let tracks: Vec<rspotify::model::FullTrack> = self.spotify
      .playlist_items(p_id, None, None)
      .filter_map(|res| async move {
        res.inspect_err(|e| log::warn!("Error getting playlist items: {}", e)).ok()
      } )
      .filter_map(|item| async move {
        if item.track.is_none() {
          log::warn!("Filtered out item with no track: {:?}", item);
        }
        item.track })
      .filter_map(|item| async move {
        match item {
          PlayableItem::Track(t) => Some(t),
          PlayableItem::Episode(e) => {
            log::warn!("Filtered out an episode: {:?}", e.name);
            None
          }
        }
      })
      .filter(|item| {
        let preview_url = item.preview_url.clone(); // Clone the field to pass to the async block
        if preview_mode && preview_url.is_none() {
          log::info!("Filtered out due to missing preview_url: {:?} - {}", item.name, item.artists.first().unwrap().name);
        }
        async move { !preview_mode || preview_url.is_some() } // Use the cloned value
      })
      .collect()
      .await;
    tracks
  }

  /// Plays the song belonging to the question given by `index`
  pub async fn begin_question_action(&mut self, index: usize) -> Result<(), QuizError> {
    if index > self.songs.len() {
      Err(QuizError::RuntimeError("Invalid song index".to_string()))
    } else {
      if self.preview_mode {
        // Use song preview MP3 in preview mode
        let bytes = self.songs[index].preview_mp3.take().ok_or(QuizError::RuntimeError("No preview in preview mode".to_string()))?;
        // see https://github.com/RustAudio/rodio/issues/171, sink cannot be stopped and play sounds afterwards
        // so we have to create a new one every time
        // self.sink = Some(rodio::Sink::try_new(&self.stream_handle)?);
        // let cursor = std::io::Cursor::new(bytes);
        // let source = rodio::Decoder::new(cursor)?;
        // self.sink.as_ref().unwrap().append(source);
      } else {
        // Use a spotify player running somewhere (we take the currently active device or the first one if there is no
        // active one
        let song = &self.songs[index].song;
        let track_id = song.id.clone().ok_or(QuizError::RuntimeError("No TrackId available".to_string()))?;
        let uris: Vec<PlayableId> = vec![PlayableId::Track(track_id)]; // Convert TrackId to PlayableId::Track
        let devices = self.spotify.device().await?;
        let mut playback_device: Option<&Device> = devices.iter().find(|dev| dev.is_active);
        if playback_device.is_none() {
          playback_device = devices.first();
        }
        let device_id = playback_device
          .ok_or(QuizError::RuntimeError("No playback device".to_string()))?.id.as_ref()
          .ok_or(QuizError::RuntimeError("No id from playback device".to_string()))?
          .as_str();
        self.spotify.volume(100, Some(device_id)).await?;
        self.spotify.start_uris_playback(uris,
                                         Some(device_id),
                                         None,
                                         Some(song.duration / 3)).await?;
      }
      log::info!("Begin question {} {} - {}", index, self.songs[index].song.artists.first().unwrap().name, self.songs[index].song.name);
      Ok(())
    }
  }

  /// Stops the playing songs
  pub async fn stop_question_action(&mut self, index: usize) -> Result<(), QuizError> {
    if index > self.songs.len() {
      Err(QuizError::RuntimeError("Invalid song index".to_string()))
    } else {
      // self.sink = None;
      self.spotify.pause_playback(None).await?;
      log::info!("End question {} {} - {}", index, self.songs[index].song.artists.first().unwrap().name, self.songs[index].song.name);
      Ok(())
    }
  }

  /// Get the questions generated before with `generate_questions(...)`
  pub fn get_questions(&self) -> &Vec<Question> {
    &self.questions
  }

  pub async fn shutdown(&mut self) -> Result<(), QuizError> {
    // self.sink = None;
    if self.spotify.has_token().await {
      let _ = self.spotify.pause_playback(None).await?;
      Ok(())
    } else {
      Err(QuizError::SpotifyAPIError("No spotify token"))
    }
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

  #[error("RSpotifyClientError: {0}")]
  RSpotifyClientError(#[from] rspotify::ClientError),

  #[error("RuntimeError: {0}")]
  RuntimeError(String),

  #[error("RodioPlayError: {0}")]
  RodioPlayError(#[from] rodio::PlayError),

  #[error("RodioDecoderError: {0}")]
  RodioDecoderError(#[from] rodio::decoder::DecoderError),
}
