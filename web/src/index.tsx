import React from 'react';
import ReactDOM from 'react-dom';
import './index.scss';
import reportWebVitals from './reportWebVitals';
import {BrowserRouter, Route, Routes} from 'react-router-dom';
import {GameView} from "./views/GameView/GameView";
import {GlobalStateProvider} from "./views/GlobalStateProvider/GlobalStateProvider";
import {AdminView} from "./views/AdminView/AdminView";
import {RedirectView} from "./views/RedirectView/RedirectView";
import {LoginView} from "./views/LoginView/LoginView";

const rootElement = document.getElementById('root');
ReactDOM.render(
  <GlobalStateProvider>
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<LoginView />} />
        <Route path="/game" element={<GameView/>} />
        <Route path="/control" element={<AdminView />} />
        <Route path="/redirect" element={<RedirectView />} />
      </Routes>
    </BrowserRouter>
  </GlobalStateProvider>,
  rootElement
);

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals();
