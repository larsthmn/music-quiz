import React, {useEffect, useState} from 'react';
import './App.scss';
import {LoginView} from "./views/LoginView/LoginView";
import {GameView} from "./views/GameView/GameView";
import {AdminView} from "./views/AdminView/AdminView";

const LOCALSTORE_USER = "username";
const TIME_SYNC_PERIOD = 20000;

enum UserState {
  NotLoggedIn,
  LoggedIn,
  Admin
}

function App() {
  const [username, setUsername] = useState<string | null>(window.localStorage.getItem(LOCALSTORE_USER));
  const [timediff, setTimediff] = useState<number>(0);
  const [userState, setUserState] = useState<UserState>(UserState.NotLoggedIn);

  const goBack = () => {
    setUserState(UserState.NotLoggedIn);
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

  if (userState == UserState.NotLoggedIn || username == null) {
    return (
      <LoginView username={username} onSubmit={(name: string, admin: boolean) => {
        if (admin) {
          setUserState(UserState.Admin);
        } else if (name.length > 0) {
          window.localStorage.setItem(LOCALSTORE_USER, name);
          setUsername(name);
          setUserState(UserState.LoggedIn);
        }
      }}/>
    );
  } else if (userState == UserState.LoggedIn) {
    return (
      <GameView username={username} exit={goBack} timediff={timediff}/>
    );
  } else { // } if (userState == UserState.Admin) {
    return (
      <AdminView exit={goBack} timediff={timediff}/>
    );
  }
}

export default App;
