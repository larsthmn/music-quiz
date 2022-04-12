import React, {useEffect, useState} from "react";
import './AdminView.scss';

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
                     onChange={(e) => onChange(e.target.value)}/> {sm.description}
            </label>
          );
        })}
      </div>
    );
  }

const useFetch = (url: string) => {
  const [data, setData] = useState(null);

  // empty array as second argument equivalent to componentDidMount
  useEffect(() => {
    async function fetchData() {
      const response = await fetch(url);
      const json = await response.json();
      setData(json);
    }
    fetchData();
  }, [url]);

  return data;
};

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

  if (preferences) {
    return (
      <div className="admin-container">
        <button onClick={() => {
          fetch("/start_game?playlist=" + "whatever", {
            'method': 'POST',
          }).then(r => console.log(r));
        }}>
          Spiel starten
        </button>
        <button onClick={() => {
          fetch("/stop_game", {
            'method': 'POST',
          }).then(r => console.log(r));
        }}>
          Spiel abbrechen
        </button>

        <SingleSelection selected={preferences.data.scoremode} name="scoremode" display="Punktebewertung" options={SCORE_MODES} onChange={(sm: string) => {
          setPreferences({data: {scoremode: sm}});
        }}/>

        <button onClick={() => {
          parseResponse(fetch("/set_preferences", {
            'method': 'POST',
            'headers': {
              'Content-Type': 'application/json',
            },
            'body': JSON.stringify(preferences.data)
          }));
        }}>
          Speichern
        </button>

        <button
          className={'backbutton'}
          onClick={exit}>
        </button>
      </div>);
  } else {
    return (<h1>Lade...</h1>)
  }

}