import React, {useEffect, useState} from "react";
import './AdminView.scss';
import {SingleSelection, SingleSelectionElement} from "../../components/SingleSelection";
import {spotifyLogin} from "../../spotifyLogin";
import {Link} from "react-router-dom";

enum ScoreMode {
  Time = "Time",
  WrongFalse = "WrongFalse",
  Order = "Order"
}

const SCORE_MODES: SingleSelectionElement[] = [
  {name: ScoreMode.Time, description: "Zeit"},
  {name: ScoreMode.WrongFalse, description: "Nur richtig/falsch"},
  {name: ScoreMode.Order, description: "Reihenfolge"}];

// enum Timings {
//   BetweenRounds = "BetweenRounds",
//   ToAnswer = "ToAnswer",
//   BetweenAnswers = "BetweenAnswers",
//   BeforeRound = "BeforeRound"
// }
//
// const TIME_OPTIONS: MultiSliderElement[] = [
//   {name: Timings.BetweenRounds, description: "Zeit"},
//   {name: ScoreMode.WrongFalse, description: "Nur richtig/falsch"},
//   {name: ScoreMode.Order, description: "Reihenfolge"}];

type SliderProps = {
  name: string,
  description: string,
  value: number,
  min: number,
  max: number,
  onChange: (value: number) => void,
}

// type SliderProps = {
//   options: MultiSliderElement[],
//   name: string,
//   display: string,
//   onChange: (name: string, value: number) => void,
// }

const Slider: React.FC<SliderProps> =
  ({name, description, value, min, max, onChange}) => {
    return (
      <div>
        <input type="range" value={value} name={name} min={min} max={max} step={1}
               onChange={(e) => onChange(Number(e.target.value))}/>
        {description}: {value}s
      </div>
    );
  }

type GamePreferences = {
  scoremode: string,
  playlists: string[],
  selected_playlist: string,
  time_to_answer: number,
  time_between_answers: number,
  time_before_round: number,
}

export const AdminView: React.FC = () => {
  const [preferences, setPreferences] = useState<GamePreferences | null>(null);

  const parseResponse = (promise: Promise<Response>) => {
    promise.then((response) => response.json(), () => {
      console.log("error on parsing json");
    })
      .then((data: GamePreferences) => {
        setPreferences(data);
      }, () => {
        console.log("error on getting preferences");
      });
  }

  useEffect(() => {
    parseResponse(fetch("/get_preferences"));
    const interval = setInterval(() => parseResponse(fetch("/get_preferences")), 2000);
    return () => clearInterval(interval);
  }, [])

  const savePreference = (name: string, value: string) => {
    parseResponse(
      fetch("/set?" + name + "=" + value, {
        'method': 'POST'
      })
    );
  };

  const startGame = () => {
    fetch("/start_game", {
      'method': 'POST',
    }).then(r => console.log(r));
  }

  const stopGame = () => {
    fetch("/stop_game", {
      'method': 'POST',
    }).then(r => console.log(r));
  };

  if (preferences) {
    return (
      <div className="admin-container">
        <button onClick={startGame}>
          Spiel starten
        </button>
        <button onClick={stopGame}>
          Spiel abbrechen
        </button>
        <SingleSelection selected={preferences.scoremode}
                         name="scoremode" display="Punktebewertung"
                         options={SCORE_MODES} onChange={(s) => savePreference("scoremode", s)}/>

        <div>
          Playlist:
          <select value={preferences.selected_playlist} onChange={(e) => savePreference("playlist", e.target.value)}>
            {preferences.playlists.map((p) => {
              return <option key={p} value={p}>{p}</option>
            })}
          </select>
        </div>

        <div>
          <Slider name={"time_to_answer"} description={"Zeit zum Antworten"} value={preferences.time_to_answer} min={3}
                  max={45} onChange={(v) => savePreference("time_to_answer", String(v))}/>
          <Slider name={"time_between_answers"} description={"Zeit zwischen Antworten"}
                  value={preferences.time_between_answers} min={0}
                  max={45} onChange={(v) => savePreference("time_between_answers", String(v))}/>
          <Slider name={"time_before_round"} description={"Zeit vor Rundenstart"} value={preferences.time_before_round}
                  min={0}
                  max={20} onChange={(v) => savePreference("time_before_round", String(v))}/>
        </div>

        <button onClick={(e) => {
          e.preventDefault();
          spotifyLogin()
        }}>
          Spotify verbinden
        </button>

        <Link to='/'>
          <button className={'backbutton'} />
        </Link>
      </div>);
  } else {
    return (<h1>Lade...</h1>)
  }

}