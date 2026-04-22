import express from 'express';
import { askGrok } from './grok-reverse-api-grok-main.js';

const app = express();
app.use(express.json()); // Allows us to read JSON from Claude
const MAX_PORT_TRIES = 10;

// The API Endpoint Claude will talk to
app.post('/ask-grok', async (req, res) => {
    const userPrompt = req.body.prompt;
    if (!userPrompt) return res.status(400).json({ error: "No prompt provided" });

    try {
        const grokResponse = await askGrok(userPrompt);
        res.json({ result: grokResponse });
    } catch (error) {
        res.status(500).json({
            error: 'Grok request failed',
            details: error.message
        });
    }
});

const START_PORT = Number(process.env.PORT) || 3005;

function listenWithRetry(port, attempt = 1) {
    const server = app.listen(port, () => {
        console.log(`🍺 The Tavern is open! Reverse API running on http://localhost:${port}`);
    });

    server.on('error', (error) => {
        if (error.code === 'EADDRINUSE') {
            if (attempt >= MAX_PORT_TRIES) {
                console.error(
                    `Server failed to start: no open port found after ${MAX_PORT_TRIES} attempts starting at ${START_PORT}.`
                );
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

listenWithRetry(START_PORT);