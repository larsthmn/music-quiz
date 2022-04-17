import React, {useEffect, useState} from "react";
import './AdminView.scss';
import {Simulate} from "react-dom/test-utils";
import play = Simulate.play;

type AdminViewProps = {
  timediff: number,
  exit: () => void
}

type Selection = { // todo: is there a builtin-type for that like a map?
  name: string,
  description: string,
}

const SCORE_MODES: Selection[] = [
  {name: "Time", description: "Zeit"},
  {name: "WrongFalse", description: "Nur richtig/falsch"},
  {name: "Order", description: "Reihenfolge"}];

type SelectionProps = {
  options: Selection[],
  selected: string,
  name: string,
  display: string,
  onChange: (selected: string) => void
}

const SingleSelection: React.FC<SelectionProps> =
  ({options, selected,  name, display, onChange}) => {
    return (
      <div className="radio-container">
        <h3>{display}</h3>
        {options.map((sm) => {
          console.log(name + ", selected:" + selected);
          return (
            <label>
              <input checked={selected === sm.name} type="radio" value={sm.name} name={name}
                     onClick={() => onChange(sm.name)}/> {sm.description}
            </label>
          );
        })}
      </div>
    );
  }

export const AdminView: React.FC<AdminViewProps> = ({timediff, exit}) => {
  const [preferences, setPreferences] = useState({data: {scoremode: "Time"}});

  const parseResponse = (promise: Promise<Response>) => {
    promise.then((response) => response.json(), () => {
      console.log("error on parsing json");
    })
      .then((data) => {
        setPreferences({data: data});
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
        <SingleSelection selected={preferences.data.scoremode}
                         name="scoremode" display="Punktebewertung"
                         options={SCORE_MODES} onChange={(s) => savePreference("scoremode", s)} />

        <select onChange={(e) => savePreference("playlist", e.target.value)}>
          <option value="1">Playlist 1</option>
          <option value="2">Playlist 2</option>
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