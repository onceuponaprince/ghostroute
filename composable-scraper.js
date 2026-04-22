import { chromium } from 'playwright';

// 1. THE GAME ENGINE (Base Class)
class BaseScraper {
    browser = null;

    constructor() {
        console.log("Initializing the Scraper Engine...");
    }

    async init() {
        this.browser = await chromium.launch({ headless: true });
    }

    async execute(url, extractionLogic) {
        const page = await this.browser.newPage();
        try {
            await page.goto(url, { waitUntil: 'domcontentloaded' });
            // We pass the page object into the specific logic function
            const data = await extractionLogic(page);
            return data;
        } catch (e) {
            console.error(`[Engine Fault] Failed at ${url}:`, e);
        } finally {
            await page.close();
        }
    }

    async shutdown() {
        if (this.browser) await this.browser.close();
    }
}

// 2. THE EXPANSION PACKS (Site-Specific Logic)

// Logic for scraping a news site
const HackerNewsStrategy = async (page) => {
    // Wait for the specific element to load
    await page.waitForSelector('.titleline > a');
    // Run Javascript directly inside the browser console to map the data
    return await page.$$eval('.titleline > a', links => 
        links.map(a => ({ title: a.innerText, url: a.href }))
    );
};

// Logic for scraping an e-commerce site
const AmazonStrategy = async (page) => {
    await page.waitForSelector('#productTitle');
    const title = await page.$eval('#productTitle', el => el.innerText.trim());
    const price = await page.$eval('.a-price .a-offscreen', el => el.innerText.trim()).catch(() => "Price hidden");
    return { title, price };
};

// 3. THE MAIN LOOP
async function runSwarm() {
    const engine = new BaseScraper();
    await engine.init();

    console.log("Scraping Hacker News...");
    const news = await engine.execute('https://news.ycombinator.com', HackerNewsStrategy);
    console.log(news[0]); // Print top story

    console.log("Scraping Amazon...");
    const product = await engine.execute('https://www.amazon.com/dp/B08N5WRWNW', AmazonStrategy);
    console.log(product);

    await engine.shutdown();
}

await runSwarm();