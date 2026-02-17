import path from 'node:path';
import { processFont } from '@/utils/font';

const assetsDir = path.resolve(import.meta.dir, '../../../crates/editor/assets');

const PHANTOM_FONTS = ['Noto-Phantom', 'Noto-Phantom-Emoji'];

for (const name of PHANTOM_FONTS) {
  const ttfPath = path.join(assetsDir, `${name}.ttf`);
  const ttfData = new Uint8Array(await Bun.file(ttfPath).bytes());

  console.log(`Processing ${name}...`);
  const { base } = await processFont(name, ttfData);

  const outPath = path.join(assetsDir, `${name}.bin`);
  await Bun.write(outPath, base);
  console.log(`  Written: ${outPath} (${(base.length / 1024).toFixed(1)}KB)`);
}
