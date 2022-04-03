import React, {useEffect, useState} from 'react';
import './App.scss';
import {LoginView} from "./views/LoginView/LoginView";
import {GameView} from "./views/GameView/GameView";

const LOCALSTORE_USER = "username";
const TIME_SYNC_PERIOD = 20000;

function App() {
  const [username, setUsername] = useState<string | null>(window.localStorage.getItem(LOCALSTORE_USER));
  const [timediff, setTimediff] = useState<number>(0);

  const goBack = () => {
    setUsername("");
    window.localStorage.removeItem(LOCALSTORE_USER);
  }

  // time sync
  useEffect(() => {
    const interval = setInterval(() => {
      const now = Date.now();
      fetch("/get_time?now=" + now)
        .then((response) => response.json(), () => {
          console.log("error on parsing json");
        })
        .then((data) => {
          console.log("timediff " + data.diff_receive + "ms");
          // todo: better time synch, use roundtrip time or something
          setTimediff(data.diff_receive);
        }, () => {
          console.log("error on getting time");
        });
    }, TIME_SYNC_PERIOD);
    return () => clearInterval(interval);
  })

  // No username set => needs login
  if (!username) {
    return (
      <LoginView onSubmit={(name: string) => {
        window.localStorage.setItem(LOCALSTORE_USER, name);
        setUsername(name);
      }}/>
    );
  }

  // Username set = Ready for game
  return (
    <GameView username={username} exit={goBack} timediff={timediff}/>
  )
}

export default App;
