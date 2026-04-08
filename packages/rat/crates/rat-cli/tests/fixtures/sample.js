const express = require('express');

class Router {
  constructor(app) {
    this.app = app;
  }

  register(path, handler) {
    this.app.get(path, handler);
  }
}

function createApp() {
  const app = express();
  return new Router(app);
}

const handler = (req, res) => {
  res.json({ status: 'ok' });
};
