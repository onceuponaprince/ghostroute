import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { createJobStore } from './jobs.js';

describe('jobs — in-memory store', () => {
  let store;

  beforeEach(() => {
    vi.useFakeTimers();
    store = createJobStore({ ttlMs: 24 * 60 * 60 * 1000 });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('create returns a job with id and queued status', () => {
    const job = store.create();
    expect(typeof job.jobId).toBe('string');
    expect(job.jobId.length).toBeGreaterThan(10);
    expect(store.get(job.jobId)).toMatchObject({ status: 'queued' });
  });

  it('updateProgress transitions queued → running with progress text', () => {
    const { jobId } = store.create();
    store.updateProgress(jobId, 'Searching sources');
    expect(store.get(jobId)).toMatchObject({ status: 'running', progress: 'Searching sources' });
  });

  it('complete stores the result and sets status done', () => {
    const { jobId } = store.create();
    store.complete(jobId, { answer: 'a', sources: [], threadId: 't' });
    expect(store.get(jobId)).toMatchObject({ status: 'done', result: { answer: 'a', sources: [], threadId: 't' } });
  });

  it('fail stores the error and sets status failed', () => {
    const { jobId } = store.create();
    store.fail(jobId, new Error('boom'));
    expect(store.get(jobId)).toMatchObject({ status: 'failed', error: 'boom' });
  });

  it('get returns undefined for unknown id', () => {
    expect(store.get('no-such-id')).toBeUndefined();
  });

  it('completed jobs expire after TTL', () => {
    const { jobId } = store.create();
    store.complete(jobId, { answer: 'x' });
    vi.advanceTimersByTime(24 * 60 * 60 * 1000 + 1);
    expect(store.get(jobId)).toBeUndefined();
  });

  it('running jobs do NOT expire (only completed/failed do)', () => {
    const { jobId } = store.create();
    store.updateProgress(jobId, 'working');
    vi.advanceTimersByTime(24 * 60 * 60 * 1000 + 1);
    expect(store.get(jobId)).toMatchObject({ status: 'running' });
  });
});
