import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { processFont } from '#/utils/font.ts';

const assetsDir = path.resolve(import.meta.dirname, '../../../crates/editor/assets');

const PHANTOM_FONTS = ['Noto-Phantom', 'Noto-Phantom-Emoji'];

for (const name of PHANTOM_FONTS) {
  const ttfPath = path.join(assetsDir, `${name}.ttf`);
  const ttfData = new Uint8Array(await readFile(ttfPath));

  console.log(`Processing ${name}...`);
  const { base } = await processFont(name, ttfData);

  const outPath = path.join(assetsDir, `${name}.bin`);
  await writeFile(outPath, base);
  console.log(`  Written: ${outPath} (${(base.length / 1024).toFixed(1)}KB)`);
}
