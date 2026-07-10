const puppeteer = require('puppeteer-core');
const fs = require('fs');
const path = require('path');

const CHROME_PATHS = [
  process.env.CHROME_BIN,
  '/usr/bin/google-chrome-stable',
  '/usr/bin/google-chrome',
  '/usr/bin/chromium',
  '/usr/bin/chromium-browser',
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
];

function findChrome() {
  for (const p of CHROME_PATHS) {
    if (p && fs.existsSync(p)) return p;
  }
  throw new Error(
    'Could not find Chrome/Chromium. Set CHROME_BIN or install Chrome.'
  );
}

async function main() {
  const input = process.argv[2] || 'pitch-deck.html';
  const output = process.argv[3] || 'fluidic-pitch-deck.pdf';
  const inputPath = path.resolve(input);

  if (!fs.existsSync(inputPath)) {
    console.error(`Input file not found: ${inputPath}`);
    process.exit(1);
  }

  const browser = await puppeteer.launch({
    headless: 'new',
    executablePath: findChrome(),
    args: ['--no-sandbox', '--disable-setuid-sandbox'],
  });

  const page = await browser.newPage();
  await page.goto(`file://${inputPath}`, { waitUntil: 'networkidle0' });

  // Wait for fonts and any async rendering.
  await new Promise((resolve) => setTimeout(resolve, 1000));

  await page.pdf({
    path: output,
    printBackground: true,
    preferCSSPageSize: true,
  });

  await browser.close();
  console.log(`PDF generated: ${path.resolve(output)}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
