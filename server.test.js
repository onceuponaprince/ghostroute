import { describe, it, expect, vi, beforeEach } from 'vitest';
import request from 'supertest';
import { createJobStore } from './providers/perplexity/jobs.js';

vi.mock('./providers/perplexity/index.js', () => ({
  askPerplexity: vi.fn(),
  askPerplexityDeep: vi.fn(),
}));

// Stub Grok so importing server.js doesn't launch a browser during tests
vi.mock('./grok-reverse-api-grok-main.js', () => ({
  askGrok: vi.fn(),
}));

const { askPerplexity } = await import('./providers/perplexity/index.js');
const { app } = await import('./server.js');

describe('POST /ask-perplexity', () => {
  beforeEach(() => vi.clearAllMocks());

  it('returns structured result on success', async () => {
    askPerplexity.mockResolvedValueOnce({
      answer: 'Mark Zuckerberg founded Meta.',
      sources: [{ index: 1, title: 'Wikipedia', url: 'https://en.wikipedia.org/wiki/Meta', domain: 'en.wikipedia.org' }],
      threadId: 'abc',
    });
    const res = await request(app)
      .post('/ask-perplexity')
      .send({ prompt: 'who founded meta?' });
    expect(res.status).toBe(200);
    expect(res.body.answer).toMatch(/Zuckerberg/);
    expect(res.body.sources[0].url).toMatch(/^https?:\/\//);
    expect(res.body.threadId).toBe('abc');
  });

  it('returns 400 on missing prompt', async () => {
    const res = await request(app).post('/ask-perplexity').send({});
    expect(res.status).toBe(400);
  });

  it('returns 401 on PerplexityAuthError', async () => {
    const { PerplexityAuthError } = await import('./providers/perplexity/errors.js');
    askPerplexity.mockRejectedValueOnce(new PerplexityAuthError());
    const res = await request(app).post('/ask-perplexity').send({ prompt: 'x' });
    expect(res.status).toBe(401);
    expect(res.body.error).toBe('PerplexityAuthError');
  });

  it('forwards model, tool, focus, threadId, raw to askPerplexity', async () => {
    askPerplexity.mockResolvedValueOnce({ answer: 'ok', sources: [], threadId: 't' });
    await request(app)
      .post('/ask-perplexity')
      .send({
        prompt: 'x',
        model: 'claude',
        tool: 'deep-research',
        focus: 'academic',
        threadId: 'continued',
        raw: true,
      });
    expect(askPerplexity).toHaveBeenCalledWith(expect.objectContaining({
      prompt: 'x',
      model: 'claude',
      tool: 'deep-research',
      focus: 'academic',
      threadId: 'continued',
      raw: true,
    }));
  });
});

describe('POST /ask-perplexity/deep + GET /ask-perplexity/deep/:jobId', () => {
  beforeEach(() => vi.clearAllMocks());

  it('POST returns 202 with jobId', async () => {
    const { askPerplexityDeep } = await import('./providers/perplexity/index.js');
    askPerplexityDeep.mockReturnValueOnce({ jobId: 'fake-uuid-123' });

    const res = await request(app)
      .post('/ask-perplexity/deep')
      .send({ prompt: 'what are the latest advancements in fusion energy?' });

    expect(res.status).toBe(202);
    expect(res.body.jobId).toBe('fake-uuid-123');
  });

  it('POST returns 400 on missing prompt', async () => {
    const res = await request(app).post('/ask-perplexity/deep').send({});
    expect(res.status).toBe(400);
  });

  it('GET returns 404 for unknown jobId', async () => {
    const res = await request(app).get('/ask-perplexity/deep/no-such-job');
    expect(res.status).toBe(404);
    expect(res.body.error).toBe('JobNotFound');
  });

  it('POST forwards model, focus, threadId, raw to askPerplexityDeep', async () => {
    const { askPerplexityDeep } = await import('./providers/perplexity/index.js');
    askPerplexityDeep.mockReturnValueOnce({ jobId: 'x' });
    await request(app)
      .post('/ask-perplexity/deep')
      .send({ prompt: 'x', model: 'claude', focus: 'academic', threadId: 't', raw: true });
    expect(askPerplexityDeep).toHaveBeenCalledWith(expect.objectContaining({
      prompt: 'x', model: 'claude', focus: 'academic', threadId: 't', raw: true,
      store: expect.anything(),
    }));
  });
});
