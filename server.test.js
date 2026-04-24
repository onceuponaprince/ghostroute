import { describe, it, expect, vi, beforeEach } from 'vitest';
import request from 'supertest';

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
