import React, {useEffect, useState} from "react";
import './AdminView.scss';
import {SingleSelection, Selection} from "../../components/SingleSelection";

enum ScoreMode {
  Time = "Time",
  WrongFalse = "WrongFalse",
  Order = "Order"
}

const SCORE_MODES: Selection[] = [
  {name: ScoreMode.Time, description: "Zeit"},
  {name: ScoreMode.WrongFalse, description: "Nur richtig/falsch"},
  {name: ScoreMode.Order, description: "Reihenfolge"}];

type GameContent = {
  playlist: string,
  count: number
}

type GamePreferences  = {
 scoremode: string,
 playlists: string[],
 selected_playlist: string,
 content: GameContent,
}

type AdminViewProps = {
  timediff: number,
  exit: () => void
}

export const AdminView: React.FC<AdminViewProps> = ({timediff, exit}) => {
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
  }, [])

  const savePreference = (name: string, value: string) => {
    parseResponse(
      fetch("/set?" + name + "=" + value, {
        'method': 'POST'
      })
    );
  };

  const startGame =  () => {
      fetch("/start_game?playlist=" + "whatever", {
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
                         options={SCORE_MODES} onChange={(s) => savePreference("scoremode", s)} />

        <select onChange={(e) => savePreference("playlist", e.target.value)}>
          {preferences.playlists.map((p) => {
            return <option selected={p === preferences.selected_playlist} key={p} value={p}>{p}</option>
          })}
        </select>

        <button
          className={'backbutton'}
          onClick={exit}>
        </button>
      </div>);
  } else {
    return (<h1>Lade...</h1>)
  }

}