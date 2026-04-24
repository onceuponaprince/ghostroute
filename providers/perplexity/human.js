// Human-behaviour helpers for Playwright interactions.
//
// Automation detection (Cloudflare Turnstile, Perplexity's own gates) looks for
// robotic patterns: instant teleport clicks, uniform keystroke intervals, zero
// idle time. These helpers add realistic jitter so interactions look like a
// person rather than a script.

export function randomDelay(minMs, maxMs) {
  return Math.floor(Math.random() * (maxMs - minMs + 1)) + minMs;
}

export async function humanPause(page, minMs, maxMs) {
  await page.waitForTimeout(randomDelay(minMs, maxMs));
}

// Typo characters near common letters on QWERTY — cheap approximation of
// fat-finger mistakes. Not cryptographically real but looks plausible.
const NEAR_KEY = {
  a: 's', s: 'd', d: 'f', f: 'g', g: 'h', h: 'j', j: 'k', k: 'l',
  q: 'w', w: 'e', e: 'r', r: 't', t: 'y', y: 'u', u: 'i', i: 'o', o: 'p',
  z: 'x', x: 'c', c: 'v', v: 'b', b: 'n', n: 'm',
};

// Types `text` character-by-character via `page.keyboard`, with per-char delay
// jitter, occasional pauses, and a small rate of typo-then-backspace
// corrections. Assumes focus is already on the target input.
export async function humanType(page, text, { typoChance = 0.025 } = {}) {
  for (const char of text) {
    const lower = char.toLowerCase();
    const nearby = NEAR_KEY[lower];
    if (nearby && Math.random() < typoChance) {
      // Type a wrong char, pause like noticing the typo, backspace, carry on
      const wrong = char === char.toUpperCase() ? nearby.toUpperCase() : nearby;
      await page.keyboard.type(wrong, { delay: randomDelay(45, 130) });
      await page.waitForTimeout(randomDelay(160, 420));
      await page.keyboard.press('Backspace');
      await page.waitForTimeout(randomDelay(80, 200));
    }
    await page.keyboard.type(char, { delay: randomDelay(45, 130) });
    if (Math.random() < 0.08) {
      await page.waitForTimeout(randomDelay(120, 320));
    }
  }
}

// Moves the mouse to the locator's box with a few intermediate waypoints
// (Playwright's default `.click()` teleports; Bezier-ish paths are closer to
// real input), hovers briefly, then clicks with a non-zero mousedown→up delay.
export async function humanClick(page, locator, { timeout = 10_000 } = {}) {
  const handle = await locator.first().elementHandle({ timeout });
  if (!handle) throw new Error('humanClick: element not found');
  const box = await handle.boundingBox();
  if (!box) {
    // Element has no box (display:none or off-screen) — fall back to .click()
    await locator.first().click({ timeout, delay: randomDelay(40, 110) });
    return;
  }

  const targetX = box.x + box.width / 2 + randomDelay(-Math.max(1, box.width / 8), Math.max(1, box.width / 8));
  const targetY = box.y + box.height / 2 + randomDelay(-Math.max(1, box.height / 8), Math.max(1, box.height / 8));

  // Move through 2-3 intermediate points for a non-linear path
  const steps = 2 + Math.floor(Math.random() * 2);
  const current = await page.evaluate(() => ({ x: window.innerWidth / 2, y: window.innerHeight / 2 }));
  for (let i = 1; i <= steps; i++) {
    const t = i / (steps + 1);
    const jitterX = randomDelay(-12, 12);
    const jitterY = randomDelay(-12, 12);
    const x = current.x + (targetX - current.x) * t + jitterX;
    const y = current.y + (targetY - current.y) * t + jitterY;
    await page.mouse.move(x, y, { steps: 4 });
    await page.waitForTimeout(randomDelay(20, 80));
  }
  await page.mouse.move(targetX, targetY, { steps: 6 });
  await page.waitForTimeout(randomDelay(100, 280));
  await page.mouse.down();
  await page.waitForTimeout(randomDelay(40, 110));
  await page.mouse.up();
}
