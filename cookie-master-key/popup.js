/**
 * Converts Chrome sameSite values to Playwright format.
 * @param {string} chromeSameSite - The sameSite value from Chrome
 * @returns {string} The converted sameSite value for Playwright
 */
function convertSameSite(chromeSameSite) {
  if (chromeSameSite === "no_restriction") {
    return "None";
  }
  if (chromeSameSite === "lax") {
    return "Lax";
  }
  return "Strict";
}

document.getElementById('export').addEventListener('click', async () => {
  const status = document.getElementById('status');

  try {
    // 1. Get the current active tab's URL.
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (!/^https?:\/\//.test(tab?.url || '')) {
      status.innerText = 'Open a normal website tab first.';
      return;
    }

    const url = new URL(tab.url);

    // 2. Fetch every cookie that would be sent with a request to this URL.
    //    Using { url } instead of { domain } is load-bearing: the domain filter
    //    only matches the exact hostname and its subdomains, which skips
    //    parent-domain cookies (e.g. .google.com when the tab is on
    //    gemini.google.com). Parent-domain cookies carry consent state
    //    (CONSENT, SOCS) and Google session cookies (SID, SSID, __Secure-*),
    //    which are what authenticated scrapers actually need.
    chrome.cookies.getAll({ url: tab.url }, (cookies) => {
      // 3. Convert Chrome cookie shape to Playwright cookie shape.
      const playwrightCookies = cookies.map((c) => ({
        name: c.name,
        value: c.value,
        domain: c.domain,
        path: c.path,
        expires: c.expirationDate || -1,
        httpOnly: c.httpOnly,
        secure: c.secure,
        sameSite: convertSameSite(c.sameSite)
      }));

      // 4. Save into browser downloads as <hostname>-cookies.json.
      const blob = new Blob([JSON.stringify(playwrightCookies, null, 2)], { type: 'application/json' });
      const blobUrl = URL.createObjectURL(blob);
      const filename = `${url.hostname}-cookies.json`;

      chrome.downloads.download({
        url: blobUrl,
        filename,
        saveAs: true
      }, (downloadId) => {
        URL.revokeObjectURL(blobUrl);

        if (chrome.runtime.lastError || !downloadId) {
          status.innerText = `Export failed: ${chrome.runtime.lastError?.message || 'unknown error'}`;
          return;
        }

        status.innerText = `Exported ${playwrightCookies.length} cookies.`;
      });
    });
  } catch (error) {
    status.innerText = `Export failed: ${error.message}`;
  }
});