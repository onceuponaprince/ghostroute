import { chromium } from 'playwright';
import fs from 'node:fs';

function randomDelay(minMs, maxMs) {
    return Math.floor(Math.random() * (maxMs - minMs + 1)) + minMs;
}

async function humanPause(page, minMs, maxMs) {
    await page.waitForTimeout(randomDelay(minMs, maxMs));
}

async function humanType(page, locator, text) {
    for (const char of text) {
        await locator.type(char, { delay: randomDelay(45, 130) });
        if (Math.random() < 0.08) {
            await humanPause(page, 120, 320);
        }
    }
}

export async function askGrok(prompt) {
    console.log("[Boss Fight Initiated] Equipping Stealth Chromium...");

    // 1. Launch the headless browser (The Mecha Suit)
    const browser = await chromium.launch({ headless: true });
    const context = await browser.newContext();

    // 2. Load the save file (Injecting your X cookies)
    // If you don't do this, X drops you at the login screen and you die instantly.
    const cookies = JSON.parse(fs.readFileSync('./grok.com-cookies.json', 'utf8'));
    await context.addCookies(cookies);

    const page = await context.newPage();

    try {
        console.log("[Infiltrating] Navigating to Grok...");
        await page.goto('https://grok.com', { waitUntil: 'networkidle' });
        await humanPause(page, 500, 1300);

        // 3. The Combat Sequence (DOM Manipulation)
        // We find the chat box. X changes their CSS classes constantly to mess with us,
        // so we target the placeholder text or aria-labels instead of class names.
        const chatInput = page.locator('div[contenteditable="true"][tabindex="0"]');
        await chatInput.click();
        await humanPause(page, 250, 700);
        await humanType(page, chatInput, prompt);
        await humanPause(page, 300, 900);
        await page.keyboard.press('Enter');

        console.log("[Attack Landed] Waiting for Grok's response...");

        // 4. The Waiting Game
        // We wait for the 'generating' animation to stop. 
        // We target the latest message bubble in the DOM.
        const responseLocator = page.locator('div[class*="response-content-markdown"]').last(); 
        await responseLocator.waitFor({ state: 'visible', timeout: 30000 });

        const hasParagraphTag = await responseLocator.evaluate((el) => el.innerHTML.trim().includes('<p'));
        if (hasParagraphTag === true) {
            // Grab the text only when paragraph content is present.
            const answer = await responseLocator.innerText();
            console.log("[Victory] Grok replied:", answer);
            return answer;
        }

        throw new Error('Response div does not contain a <p> tag yet.');

    } catch (error) {
        console.error("[WASTED] The bouncer caught us:", error.message);
    } finally {
        await browser.close();
    }
}

// CLI entrypoint: node grok-reverse-api-grok-main.js "your prompt"
const isMainModule = process.argv[1] && new URL(`file://${process.argv[1]}`).href === import.meta.url;
if (isMainModule) {
    const cliPrompt = process.argv.slice(2).join(' ').trim();
    if (cliPrompt) {
        try {
            await askGrok(cliPrompt);
        } catch (error) {
            console.error('[FATAL] Script crashed:', error.message);
            process.exitCode = 1;
        }
    } else {
        console.log('Usage: node grok-reverse-api-grok-main.js "your prompt"');
    }
}