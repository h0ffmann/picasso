// Prints whitepaper.html to the repo root's PICASSOPAPER.pdf with headless Chromium, the way
// site/plan/hosting-plan.pdf was made. Needs network for Google Fonts.
//   node render.mjs            (Playwright on the module path; CHROMIUM=<path> to pick a binary)
import { chromium } from 'playwright';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
import { existsSync, readFileSync } from 'node:fs';

const here = path.dirname(fileURLToPath(import.meta.url));
const browser = await chromium.launch(process.env.CHROMIUM ? { executablePath: process.env.CHROMIUM } : {});
const page = await browser.newPage();
await page.goto(pathToFileURL(path.join(here, 'whitepaper.html')).href, { waitUntil: 'networkidle' });
await page.evaluate(() => document.fonts.ready);
// doi.txt is written by `zenodo draft` once Zenodo has reserved the DOI.
const doiFile = path.join(here, 'doi.txt');
if (existsSync(doiFile)) {
  const doi = readFileSync(doiFile, 'utf8').trim();
  await page.evaluate((d) => { document.querySelector('[data-doi]').textContent = `https://doi.org/${d}`; }, doi);
}
const foot = 'font:7.5pt Inter,sans-serif;color:#4F5868;width:100%;padding:0 17mm;display:flex;justify-content:space-between';
await page.pdf({
  path: path.join(here, '..', '..', '..', 'PICASSOPAPER.pdf'),
  format: 'A4',
  printBackground: true,
  preferCSSPageSize: true,
  displayHeaderFooter: true,
  headerTemplate: '<span></span>',
  footerTemplate: `<div style="${foot}"><span>h0ffmann/picasso · same inputs, same factory · 12 October 2026</span><span><span class="pageNumber"></span> / <span class="totalPages"></span></span></div>`,
});
await browser.close();
