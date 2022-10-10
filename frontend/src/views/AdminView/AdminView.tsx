import React, {useEffect, useState} from "react";
import './AdminView.scss';
import {SingleSelection, SingleSelectionElement} from "../../components/SingleSelection";
import {spotifyLogin} from "../../spotifyLogin";
import {Link} from "react-router-dom";
import {GamePreferences} from "../../../../shared/GamePreferences";

enum ScoreMode {
  TimeLinear = "TimeLinear",
  TimeFunction = "TimeFunction",
  WrongFalse = "WrongFalse",
  Order = "Order"
}

const SCORE_MODES: SingleSelectionElement[] = [
  {name: ScoreMode.TimeFunction, description: "Zeit (Funktion)"},
  {name: ScoreMode.TimeLinear, description: "Zeit (linear)"},
  {name: ScoreMode.WrongFalse, description: "Nur richtig/falsch"},
  {name: ScoreMode.Order, description: "Reihenfolge"}];

type SliderProps = {
  name: string,
  description: string,
  value: number,
  min: number,
  max: number,
  onChange: (value: number) => void,
  unit: string
}

const Slider: React.FC<SliderProps> =
  ({name, description, value, min, max, onChange, unit}) => {
    return (
      <div>
        <input type="range" value={value} name={name} min={min} max={max} step={1}
               onChange={(e) => onChange(Number(e.target.value))}/>
        {description}: {value}{unit}
      </div>
    );
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

  const refreshSpotify = () => {
    fetch("/refresh_spotify", {
      'method': 'POST',
    }).then(r => console.log(r));
  };

  if (preferences) {
    return (
      <div className="admin-container">
        <fieldset>
          <legend>Steuerung</legend>
          <select value={preferences.selected_playlist?.id} onChange={(e) => savePreference("playlist", e.target.value)}>
            {preferences.playlists.map((p) => {
              return <option key={p.id} value={p.id}>{p.name}</option>
            })}
          </select>
          <button onClick={startGame}>
            Spiel starten
          </button>
          <button onClick={stopGame}>
            Spiel abbrechen
          </button>
        </fieldset>

        <fieldset>
          <legend>Spotify</legend>
          <div>
            <button onClick={refreshSpotify}>
              Refresh Playlists
            </button>
            <button onClick={(e) => {
              e.preventDefault();
              spotifyLogin()
            }}>
              Spotify verbinden
            </button>
          </div>
          <label>
            <input checked={preferences.preview_mode}
                   type="checkbox"
                   onChange={() => savePreference("preview_mode", String(!preferences.preview_mode))}/>
            Preview-MP3s nutzen
          </label>

        </fieldset>

        <fieldset>
          <legend>Antworten</legend>
          <div>
            <Slider name={"time_to_answer"} description={"Zeit zum Antworten"} value={preferences.time_to_answer} min={3}
                    max={30} unit="s" onChange={(v) => savePreference("time_to_answer", String(v))}/>
            <Slider name={"time_between_answers"} description={"Zeit zwischen Antworten"}
                    value={preferences.time_between_answers} min={0}
                    max={30} unit="s" onChange={(v) => savePreference("time_between_answers", String(v))}/>
            <Slider name={"time_before_round"} description={"Zeit vor Rundenstart"} value={preferences.time_before_round}
                    min={0}
                    max={20} unit="s" onChange={(v) => savePreference("time_before_round", String(v))}/>
            <Slider name={"rounds"} description={"Anzahl Runden"} value={preferences.rounds}
                    min={1}
                    max={30} unit="" onChange={(v) => savePreference("rounds", String(v))}/>
          </div>

          <div className="checkbox-container">
            <label>
              <input checked={preferences.hide_answers}
                     type="checkbox"
                     onChange={() => savePreference("hide_answers", String(!preferences.hide_answers))}/>
              Antworten bis Auflösung verbergen
            </label>
            <label>
              <input checked={preferences.ask_for_artist}
                     type="checkbox"
                     onChange={() => savePreference("ask_for_artist", String(!preferences.ask_for_artist))}/>
              Nach Künstler fragen
            </label>
            <label>
              <input checked={preferences.ask_for_title}
                     type="checkbox"
                     onChange={() => savePreference("ask_for_title", String(!preferences.ask_for_title))}/>
              Nach Titel fragen
            </label>
          </div>
        </fieldset>

        <fieldset>
          <legend>Punktebewertung</legend>
          <SingleSelection selected={preferences.scoremode}
                           name="scoremode" display="Punktebewertung"
                           options={SCORE_MODES} onChange={(s) => savePreference("scoremode", s)}/>
        </fieldset>





        <Link to='/'>
          <button className={'backbutton'} />
        </Link>
      </div>);
  } else {
    return (<h1>Lade...</h1>)
  }

}