import fs from 'node:fs/promises';
import { $ } from 'execa';

await fs.rm('dist/layers', { recursive: true, force: true });
await fs.mkdir('dist/layers/sharp/nodejs', { recursive: true });

const $$ = $({ cwd: 'dist/layers/sharp' });

await $$`tar xJf ../../../../../vendor/sharp-al2023.tar.xz`;
await $$`mv node_modules nodejs`;
await $$`zip -r ../sharp.zip .`;

await fs.rm('dist/layers/sharp', { recursive: true });

console.log('Sharp layer created');
