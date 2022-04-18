import React, {useEffect} from "react"

export const SpotifyLogin = () => {
  const REACT_APP_CLIENT_ID = 'd071021f312148b38eaa0243f11a52c8'
  const REACT_APP_REDIRECT_URL = "http://localhost:3000/redirect"
  const scope = 'playlist-read-private user-read-private user-read-email user-read-playback-state user-top-read playlist-read-private';

  const generateRandomString = function (length: number) {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';

    for (var i = 0; i < length; i++) {
      text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
  };

  const handleLogin = () => {
    // window.location = url;
    window.location.assign(url);
  };

  const state = generateRandomString(16);
  const url = 'https://accounts.spotify.com/authorize'
    + '?response_type=token'
    + '&client_id=' + encodeURIComponent(REACT_APP_CLIENT_ID)
    + '&scope=' + encodeURIComponent(scope)
    + '&redirect_uri=' + encodeURIComponent(REACT_APP_REDIRECT_URL)
    + '&state=' + encodeURIComponent(state);

  useEffect(() => {
    if (typeof window !== 'undefined') {
      handleLogin();
    }
  });

  return (
    <div>
    </div>
  )

}

export default SpotifyLogin;