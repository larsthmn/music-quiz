const prod = {
  WS_URL: 'ws://localhost:80/ws',
  SPOTIFY_REDIRECT_URL: "http://localhost:80/redirect",
};

const dev = {
  WS_URL: 'ws://localhost:8000/ws',
  SPOTIFY_REDIRECT_URL: "http://localhost:3000/redirect",
};

export const config = process.env.NODE_ENV === 'development' ? dev : prod;

