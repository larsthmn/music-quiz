const prod = {
  WS_URL: 'ws://localhost:80/ws',
};

const dev = {
  WS_URL: 'ws://localhost:8000/ws',
};

export const config = process.env.NODE_ENV === 'development' ? dev : prod;

