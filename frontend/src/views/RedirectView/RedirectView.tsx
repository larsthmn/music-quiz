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

  // todo: error handling

  // Received code, this can be used to request an access token
  const code = params.get('code');
  if (code) {
    console.log("received code " + code);
    fetch("/authorize_spotify?code=" + code, {
      'method': 'POST'})
      .catch((reason) => console.error("Error on getting/forwarding access token" + reason))
      .finally(() => nav('/control'));
    // spotifyGetAccessToken(code)
    //   .then(
    //     (response) => response.text())
    //   .then(
    //     (text) => sendToBackend(text))
    //   .catch((reason) => console.error("Error on getting/forwarding access token" + reason))
    //   .finally(() => nav('/control'));
  }

  // todo (maybe later): verify state is the same

  console.log(parsedHash);

  let nav = useNavigate();


  return (
    <h1>Weiterleiten...</h1>
  )
}