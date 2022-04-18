import React, {useEffect} from "react";
import {useLocation, useNavigate, useSearchParams} from "react-router-dom";
import queryString from 'query-string';
import {spotifyGetAccessToken} from "../../spotifyLogin";

export const sendToBackend = (json: string) => {
  console.log("send to backend" + json);
  return fetch("/authorize_spotify", {
    'method': 'POST',
    'headers': {
      'Content-Type': 'application/json',
    },
    'body': json
  });
}

export const RedirectView = () => {
  const {hash} = useLocation();
  const parsedHash = queryString.parse(hash);
  const [params, setSearchParams] = useSearchParams()
  console.log("Received token" + hash);
  console.log(params.get('code'));

  const error = params.get('error');
  if (error) {
    console.error("auth error: " + error)
  }

  // Received code, this can be used to request an access token
  const code = params.get('code');
  if (code) {
    console.log("received code " + code);
    spotifyGetAccessToken(code)
      .then(
        (response) => response.text())
      .then(
        (text) => sendToBackend(text))
      .catch((reason) => console.error("Error on getting/forwarding access token" + reason))
      .finally(() => nav('/control'));
  }

  // todo (maybe later): verify state is the same

  console.log(parsedHash);

  let nav = useNavigate();
  // fetch('/set?spotify_token=' + parsedHash.access_token, {'method': 'POST'})
  //   .then(
  //     () => nav('/control'),
  //     () => {
  //     console.log("Error on forwarding token to backend");
  //     nav('/control');
  //   });

  return (
    <h1>Weiterleiten...</h1>
  )
}