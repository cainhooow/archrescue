// Usage: node scripts/capture-design.cjs OUTPUT [CHROME_PATH]
// Requires Playwright. Set NODE_PATH when using an existing Playwright installation.
const { chromium } = require('playwright');
const fs = require('node:fs/promises');
const path = require('node:path');
const { pathToFileURL } = require('node:url');

const viewports = [[800, 600], [1024, 768], [1280, 720], [1920, 1080]];
const design = path.resolve(__dirname, '../design');
const output = process.argv[2];
const chrome = process.argv[3];

async function inspect(page) {
    return page.evaluate(() => {
        const styles = [];
        const styleIds = new Map();
        const elements = Array.from(document.querySelectorAll('body, body *')).flatMap(element => {
            const bounds = element.getBoundingClientRect();
            if (!(bounds.width && bounds.height)) return [];
            const computed = getComputedStyle(element);
            const values = Object.fromEntries(Array.from(computed).map(key => [key, computed[key]]));
            const key = JSON.stringify(values);
            if (!styleIds.has(key)) {
                styleIds.set(key, styles.length);
                styles.push(values);
            }
            return [{
                tag: element.tagName.toLowerCase(),
                id: element.id || undefined,
                class: element.getAttribute('class') || undefined,
                label: element.getAttribute('aria-label') || undefined,
                text: element.children.length ? undefined : element.textContent.trim().slice(0, 100),
                rect: [bounds.x, bounds.y, bounds.width, bounds.height],
                style: styleIds.get(key),
            }];
        });
        return { viewport: [innerWidth, innerHeight], scale: devicePixelRatio, styles, elements };
    });
}

async function capture() {
    if (!output) throw new Error('Pass an evidence output directory.');
    const browser = await chromium.launch({ executablePath: chrome, headless: true });
    const reference = path.join(output, 'reference');
    const geometry = path.join(output, 'geometry');
    await Promise.all([fs.mkdir(reference, { recursive: true }), fs.mkdir(geometry, { recursive: true })]);
    const manifest = { browser: browser.version(), scale: 1, pages: {}, gallery: [] };

    try {
        for (const [width, height] of viewports) {
            const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: 1 });
            for (const name of ['index', 'relatorio']) {
                const stem = `${name}-${width}x${height}`;
                await page.goto(pathToFileURL(path.join(design, `${name}.html`)).href);
                await page.evaluate(() => document.fonts.ready);
                await page.screenshot({ path: path.join(reference, `${stem}.png`) });
                manifest.pages[stem] = await inspect(page);
            }

            await page.goto(pathToFileURL(path.join(design, 'index.html')).href);
            const shots = await page.evaluate(() => window.gallery.flatMap(section => section.shots));
            if (manifest.gallery.length === 0) manifest.gallery = shots;
            await page.locator('.cover').click();
            await page.locator('#viewer-image').evaluate(image => image.decode());
            await page.screenshot({ path: path.join(reference, `viewer-${width}x${height}.png`) });
            manifest.pages[`viewer-${width}x${height}`] = await inspect(page);
            await page.keyboard.press('ArrowLeft');
            const first = await page.locator('#viewer-image').getAttribute('src');
            if (first !== shots[0].file) throw new Error('Gallery did not navigate to the first mockup.');
            await page.keyboard.press('Escape');
            if (!await page.locator('.cover').evaluate(button => button === document.activeElement)) {
                throw new Error('Gallery focus did not return to the opener.');
            }
            await page.close();
        }
        const zlib = require('node:zlib');
        await fs.writeFile(path.join(geometry, 'gallery-dom.json.gz'), zlib.gzipSync(JSON.stringify(manifest)));
    } finally {
        await browser.close();
    }
}

capture().catch(error => { console.error(error); process.exitCode = 1; });
