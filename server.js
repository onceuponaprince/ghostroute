import express from 'express';
import { askGrok } from './grok-reverse-api-grok-main.js';
import { askPerplexity } from './providers/perplexity/index.js';
import {
  PerplexityAuthError,
  PerplexityScrapeError,
  PerplexityTimeoutError,
  PerplexityParseError,
} from './providers/perplexity/errors.js';

export const app = express();
app.use(express.json());

const MAX_PORT_TRIES = 10;

app.post('/ask-grok', async (req, res) => {
  const userPrompt = req.body.prompt;
  if (!userPrompt) return res.status(400).json({ error: 'No prompt provided' });

  try {
    const grokResponse = await askGrok(userPrompt);
    res.json({ result: grokResponse });
  } catch (error) {
    res.status(500).json({
      error: 'Grok request failed',
      details: error.message,
    });
  }
});

app.post('/ask-perplexity', async (req, res) => {
  const { prompt, model, tool, focus, threadId, raw } = req.body || {};
  if (!prompt) return res.status(400).json({ error: 'No prompt provided' });

  try {
    const result = await askPerplexity({ prompt, model, tool, focus, threadId, raw });
    res.json(result);
  } catch (err) {
    return respondPerplexityError(res, err);
  }
});

function respondPerplexityError(res, err) {
  if (err instanceof PerplexityAuthError) {
    return res.status(401).json({ error: 'PerplexityAuthError', message: err.message });
  }
  if (err instanceof PerplexityTimeoutError) {
    return res.status(504).json({ error: 'PerplexityTimeoutError', stage: err.stage, timeoutMs: err.timeoutMs });
  }
  if (err instanceof PerplexityScrapeError) {
    return res.status(502).json({
      error: 'PerplexityScrapeError',
      stage: err.stage,
      selector: err.selector,
      html: err.htmlTruncated,
    });
  }
  if (err instanceof PerplexityParseError) {
    return res.status(502).json({ error: 'PerplexityParseError', reason: err.reason });
  }
  return res.status(500).json({ error: 'InternalError', message: err.message });
}

const START_PORT = Number(process.env.PORT) || 3005;

function listenWithRetry(port, attempt = 1) {
  const server = app.listen(port, () => {
    console.log(`🍺 The Tavern is open! Reverse API running on http://localhost:${port}`);
  });
  server.on('error', (error) => {
    if (error.code === 'EADDRINUSE') {
      if (attempt >= MAX_PORT_TRIES) {
        console.error(`Server failed to start: no open port found after ${MAX_PORT_TRIES} attempts starting at ${START_PORT}.`);
        process.exit(1);
      }
      const nextPort = port + 1;
      console.warn(`Port ${port} is busy, trying ${nextPort}...`);
      listenWithRetry(nextPort, attempt + 1);
      return;
    }
    console.error('Server failed to start:', error.message);
    process.exit(1);
  });
}

// Only listen when executed directly, not when imported by tests.
const isMainModule = process.argv[1] && new URL(`file://${process.argv[1]}`).href === import.meta.url;
if (isMainModule) {
  listenWithRetry(START_PORT);
}