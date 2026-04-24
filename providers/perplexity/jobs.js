import { randomUUID } from 'node:crypto';

export function createJobStore({ ttlMs = 24 * 60 * 60 * 1000 } = {}) {
  const jobs = new Map();

  function scheduleGc(jobId) {
    setTimeout(() => {
      const job = jobs.get(jobId);
      if (!job) return;
      if (job.status === 'done' || job.status === 'failed') {
        jobs.delete(jobId);
      }
    }, ttlMs).unref?.();
  }

  return {
    create() {
      const jobId = randomUUID();
      jobs.set(jobId, { jobId, status: 'queued', createdAt: Date.now() });
      return { jobId };
    },
    updateProgress(jobId, progress) {
      const job = jobs.get(jobId);
      if (!job) return;
      job.status = 'running';
      job.progress = progress;
    },
    complete(jobId, result) {
      const job = jobs.get(jobId);
      if (!job) return;
      job.status = 'done';
      job.result = result;
      delete job.progress;
      scheduleGc(jobId);
    },
    fail(jobId, err) {
      const job = jobs.get(jobId);
      if (!job) return;
      job.status = 'failed';
      job.error = err?.message ?? String(err);
      scheduleGc(jobId);
    },
    get(jobId) {
      return jobs.get(jobId);
    },
  };
}
