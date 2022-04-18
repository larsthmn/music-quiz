import React, {useEffect} from "react";
import {useLocation, useNavigate, useSearchParams} from "react-router-dom";
import queryString from 'query-string';

export const RedirectView = () => {
  const {hash} = useLocation();
  const parsedHash = queryString.parse(hash);

  console.log("Received token" + hash);
  console.log(parsedHash);

  let nav = useNavigate();
  fetch('/set?spotify_token=' + parsedHash.access_token, {'method': 'POST'})
    .then(() => nav('/control'))
    .then(() => {
      console.log("Error on forwarding token to backend");
      nav('/control');
    });

  return (
    <h1>Weiterleiten...</h1>
  )
}