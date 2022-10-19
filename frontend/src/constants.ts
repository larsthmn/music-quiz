const prod = {
  WS_PORT: 80,
  SPOTIFY_REDIRECT_URL: "http://localhost:80/redirect",
};

const dev = {
  WS_PORT: 8000,
  SPOTIFY_REDIRECT_URL: "http://localhost:3000/redirect",
};

export const config = process.env.NODE_ENV === 'development' ? dev : prod;

