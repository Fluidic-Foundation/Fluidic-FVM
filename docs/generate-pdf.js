#!/usr/bin/env node
const puppeteer = require('puppeteer-core');
const path = require('path');
const fs = require('fs');

const htmlPath = path.resolve(__dirname, 'pitch-deck.html');
const pdfPath = path.resolve(__dirname, 'fluidic-pitch-deck.pdf');

const chromePath = process.env.CHROME_BIN || '/usr/bin/google-chrome';

if (!fs.existsSync(chromePath)) {
  console.error(`Chrome not found at ${chromePath}. Set CHROME_BIN to the correct path.`);
  process.exit(1);
}

if (!fs.existsSync(htmlPath)) {
  console.error(`HTML file not found: ${htmlPath}`);
  process.exit(1);
}

(async () => {
  console.log('Launching Chrome...');
  const browser = await puppeteer.launch({
    executablePath: chromePath,
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox'],
  });

  try {
    const page = await browser.newPage();
    await page.goto('file://' + htmlPath, { waitUntil: 'networkidle0' });

    console.log('Generating PDF...');
    await page.pdf({
      path: pdfPath,
      width: '13.3333in',
      height: '7.5in',
      printBackground: true,
      preferCSSPageSize: false,
      margin: { top: 0, right: 0, bottom: 0, left: 0 },
    });

    const stats = fs.statSync(pdfPath);
    console.log(`\nPDF generated successfully:`);
    console.log(`  Path: ${pdfPath}`);
    console.log(`  Size: ${(stats.size / 1024).toFixed(1)} KB`);
  } catch (err) {
    console.error('PDF generation failed:', err.message);
    process.exitCode = 1;
  } finally {
    await browser.close();
  }
})();
