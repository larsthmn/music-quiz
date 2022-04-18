import {useNavigate} from "react-router-dom";

const REACT_APP_CLIENT_ID = 'd071021f312148b38eaa0243f11a52c8';
const REACT_APP_CLIENT_SECRET = 'e7b6900b04b74d28a08e0e56f6c84c41';
const REACT_APP_REDIRECT_URL = "http://localhost:3000/redirect";

export const spotifyLogin = () => {
  const scope = 'user-modify-playback-state user-read-playback-state user-read-currently-playing playlist-read-collaborative playlist-read-private app-remote-control streaming user-read-email user-read-private';

  const generateRandomString = function (length: number) {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';

    for (let i = 0; i < length; i++) {
      text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
  };

  const state = generateRandomString(16);
  const url = 'https://accounts.spotify.com/authorize'
    + '?response_type=code'
    + '&client_id=' + encodeURIComponent(REACT_APP_CLIENT_ID)
    + '&scope=' + encodeURIComponent(scope)
    + '&redirect_uri=' + encodeURIComponent(REACT_APP_REDIRECT_URL)
    + '&state=' + encodeURIComponent(state);
  window.location.assign(url);
}

export const spotifyGetAccessToken = (code: string) => {
  return fetch('https://accounts.spotify.com/api/token', {
    method: 'POST',
    headers: new Headers({
      'Authorization': 'Basic ' + btoa(REACT_APP_CLIENT_ID + ':' + REACT_APP_CLIENT_SECRET),
      'Content-Type': 'application/x-www-form-urlencoded'
    }),
    body: new URLSearchParams({
      'grant_type': 'authorization_code',
      'code': code,
      'redirect_uri': 'http://localhost:3000/redirect'
    })
  })
}