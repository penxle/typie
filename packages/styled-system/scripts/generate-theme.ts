import fs from 'node:fs';
import path from 'node:path';
import { generate, REPO_ROOT } from '../src/generator/index.ts';

const { outputs, failures } = await generate(REPO_ROOT);
for (const [relative, content] of Object.entries(outputs)) {
  const target = path.join(REPO_ROOT, relative);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, content);
  console.log(`wrote ${relative}`);
}
if (failures.length > 0) {
  console.error(`contrast gate: ${failures.length} failing pairs\n  ${failures.join('\n  ')}`);
  process.exitCode = 1;
}
