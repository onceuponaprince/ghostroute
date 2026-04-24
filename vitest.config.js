import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['providers/**/*.test.js', '*.test.js'],
    // Smoke tests opt in via SMOKE=1 env var; they check this at runtime and skip otherwise.
    testTimeout: 60_000,
    hookTimeout: 60_000,
  },
});
