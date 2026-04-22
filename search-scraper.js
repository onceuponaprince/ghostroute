import * as cheerio from 'cheerio';

// This is our character's basic attack. No stealth, just speed.
async function scrapeDuckDuckGo(query) {
    console.log(`[Quest Started] Searching for: ${query}`);
    
    // 1. The Approach (HTTP GET)
    // We disguise our request slightly so we don't look completely naked.
    const url = `https://html.duckduckgo.com/html/?q=${encodeURIComponent(query)}`;
    const response = await fetch(url, {
        headers: { 'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)' }
    });

    // 2. The Loot Drop (Raw HTML)
    const rawHtml = await response.text();

    // 3. The Inventory Management (Parsing)
    const $ = cheerio.load(rawHtml);
    const results = [];

    // We target the specific CSS classes where the loot lives.
    $('.result__url').each((index, element) => {
        const link = $(element).attr('href');
        if (link) results.push(link);
    });

    console.log(`[Loot Recovered] Found ${results.length} links!`);
    return results;
}

// Press Start
scrapeDuckDuckGo("how to build an AI swarm").then(console.log);