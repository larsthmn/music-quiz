import spotifyJson from "./spotify.json";
import {config} from "./constants";

// todo: use different json in devel/prod instead of config

export const spotifyLogin = () => {

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
    + '&client_id=' + encodeURIComponent(spotifyJson.client_id)
    + '&scope=' + encodeURIComponent(spotifyJson.scopes.join(" "))
    + '&redirect_uri=' + encodeURIComponent(config.SPOTIFY_REDIRECT_URL)
    + '&state=' + encodeURIComponent(state);
  window.location.assign(url);
}

export const spotifyGetAccessToken = (code: string) => {
  return fetch('https://accounts.spotify.com/api/token', {
    method: 'POST',
    headers: new Headers({
      'Authorization': 'Basic ' + btoa(spotifyJson.client_id + ':' + spotifyJson.client_secret),
      'Content-Type': 'application/x-www-form-urlencoded'
    }),
    body: new URLSearchParams({
      'grant_type': 'authorization_code',
      'code': code,
      'redirect_uri': config.SPOTIFY_REDIRECT_URL
    })
  })
}