import { spawnSync } from 'node:child_process';

const EXCLUDE_ARGS: Record<string, (path: string) => string[]> = {
  prettier: (path) => [`!${path}/**`],
  cspell: (path) => ['--exclude', `${path}/**`],
};

const [tool, ...args] = process.argv.slice(2);
const excludeArgs = EXCLUDE_ARGS[tool];
if (!excludeArgs) {
  throw new Error(`unsupported tool: ${tool}`);
}

const ls = spawnSync('turbo', ['ls', '--output=json'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'] });
if (ls.error) {
  throw ls.error;
}
if (ls.status !== 0) {
  throw new Error(`turbo ls exited with ${ls.status ?? ls.signal}`);
}

const { packages } = JSON.parse(ls.stdout) as { packages: { items: { path: string }[] } };
const result = spawnSync(tool, [...args, ...packages.items.flatMap(({ path }) => excludeArgs(path))], { stdio: 'inherit' });
if (result.error) {
  throw result.error;
}
process.exitCode = result.status ?? 1;
