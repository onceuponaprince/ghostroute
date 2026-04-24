// Perplexity DOM selectors — the fragile layer.
// Captured 2026-04-24 via offline dissect-fixtures.mjs + live sidebar-probe.mjs.
// Prefer data-testid / aria-label / role attributes over class names.
// Update here (and only here) when Perplexity ships a UI change.

// Phase icons → phase names (consumed by parse.js extractSteps).
export const PHASE_BY_ICON = {
  '#pplx-icon-blocks': 'identifying',
  '#pplx-icon-world-search': 'searching',
  '#pplx-icon-bolt': 'insights',
};

// Focus filter → entry URL path (consumed by scrape.js launchAndNavigate).
export const FOCUS_URLS = {
  web: '/',
  academic: '/academic',
  finance: '/finance',
  health: '/health',
  patents: '/patents',
};

export const SELECTORS = {
  // --- parse layer (read from scraped HTML) ---
  answerContainer: 'div[id^="markdown-content-"]',
  sourcesContainer: '[class*="group/search-side-content"]',
  sourceItem: '[class*="group/search-side-content"] a[href]',
  sourceTitle: 'span[class*="font-medium"][class*="text-foreground"]',
  sourceSnippet: 'span[class*="text-quiet"]',
  stepItem: 'button:has(div.font-sans.text-quiet.text-sm.select-none.truncate)',
  stepQuery: 'div.font-sans.text-quiet.text-sm.select-none.truncate',
  stepPhaseIcon: 'svg use',

  // --- scrape layer (interact with live page) ---
  //
  // Note: no `focusButton` / `focusOption` selectors exist. Focus filtering
  // moved to URL-based routing in the 2026-04-24 spec revision — scrape.js
  // navigates directly to `/academic`, `/finance`, etc. via FOCUS_URLS
  // instead of clicking a dropdown. See spec §Revisions.
  promptInput: 'div[contenteditable="true"][role="textbox"]',
  submitKey: 'Enter',
  // The `+` button opens the Auto/Pro/Reasoning menu.
  modeButton: 'button[aria-label="Add files or tools"]',
  // Menu items inside the `+` dropdown, keyed by visible label ("Pro", "Reasoning").
  // NOTE: `modeOption` is a function, not a string — call it: `modeOption("Pro")`.
  modeOption: (label) => `[role="menuitem"]:has-text("${label}")`,
  // Dedicated button for Deep Research — sits outside the `+` menu.
  deepResearchButton: 'button:has-text("Deep Research")',
  // "Sources" button opens the sources overlay panel during live runs.
  sourcesButton: 'button:has-text(/^sources$/i)',

  // --- scrape layer TBD (resolve during Task 11-13 smoke tests) ---
  // generatingIndicator, doneIndicator, and deepResearchProgressText need a
  // live prompt-submission to find. Leave as empty strings; scrape.js will
  // throw a clear error if its waitForCompletion hits an unset selector.
  generatingIndicator: '',
  doneIndicator: '',
  deepResearchProgressText: '',

  // Login wall: Sign-in anchor/button appears when cookies are missing/expired.
  loginWallDetector: 'a[href*="/sign-in"], button:has-text(/sign\\s*in/i)',
};
