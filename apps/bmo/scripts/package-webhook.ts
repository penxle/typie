import { execSync } from 'node:child_process';
import { cpSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const root = path.join(import.meta.dirname, '..');
const dist = path.join(root, 'dist', 'webhook');
const zipPath = path.join(root, 'dist', 'webhook.zip');

rmSync(dist, { recursive: true, force: true });
mkdirSync(dist, { recursive: true });

cpSync(path.join(root, 'src'), path.join(dist, 'src'), { recursive: true });
writeFileSync(path.join(dist, 'handler.js'), `export { handler } from './src/webhook.ts';\n`);
writeFileSync(path.join(dist, 'package.json'), JSON.stringify({ type: 'module' }));

rmSync(zipPath, { force: true });
execSync(`cd "${dist}" && zip -r "${zipPath}" .`, { stdio: 'inherit' });

console.log(`Created ${zipPath}`);
