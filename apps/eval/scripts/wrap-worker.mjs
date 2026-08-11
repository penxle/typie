// adapter-cloudflare는 scheduled 핸들러를 지원하지 않고(sveltejs/kit#13739 미릴리스), wrangler main이
// 가리키는 경로를 자기 출력지로 써서 소스 래퍼를 main에 둘 수도 없다. 그래서 빌드 후 생성 워커를
// 개명하고 cron 래퍼를 그 자리에 쓴다. 재실행에 멱등 — 이미 래퍼면 건드리지 않는다.
import { readFile, rename, writeFile } from 'node:fs/promises';

const dir = new URL('../.svelte-kit/cloudflare/', import.meta.url);
const workerPath = new URL('_worker.js', dir);
const keptPath = new URL('_sveltekit_worker.js', dir);
const ignorePath = new URL('.assetsignore', dir);

const WRAPPER = `import worker from './_sveltekit_worker.js';
import { createDb } from '../../src/lib/server/db/index.ts';
import { pollAndPush } from '../../src/lib/server/push-poll.ts';

export default {
  fetch: (request, env, ctx) => worker.fetch(request, env, ctx),
  scheduled: async (controller, env) => {
    await pollAndPush(createDb(env.DB), env);
  },
};
`;

const current = await readFile(workerPath, 'utf8');
if (!current.includes('./_sveltekit_worker.js')) {
  await rename(workerPath, keptPath);
  await writeFile(workerPath, WRAPPER);
}

// 개명본이 정적 자산으로 서빙되지 않게 막는다(.assetsignore는 공개 서빙만 제어한다).
const ignore = await readFile(ignorePath, 'utf8').catch(() => '');
if (!ignore.split('\n').includes('_sveltekit_worker.js')) {
  await writeFile(ignorePath, `${ignore.trimEnd()}\n_sveltekit_worker.js\n`);
}
