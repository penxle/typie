import assert from 'node:assert/strict';
import test from 'node:test';
import { createSpellcheck } from './spellcheck-core.ts';

type SpellcheckDependencies = Parameters<typeof createSpellcheck>[0];

const NO_ERROR = '문법 및 철자 오류가 발견되지 않았습니다.';

const response = (body: string) => `<PnuNlpSpeller><PnuErrorWordList>${body}</PnuErrorWordList></PnuNlpSpeller>`;
const noErrorXml = response(`<Error>${NO_ERROR}</Error>`);
const providerErrorXml = response('<Error>temporary provider failure</Error>');
const wordXml = response('<PnuErrorWord><nErrorIdx>0</nErrorIdx><m_nStart>0</m_nStart><m_nEnd>1</m_nEnd></PnuErrorWord>');

const createDependencies = (overrides: Partial<SpellcheckDependencies> = {}): SpellcheckDependencies => ({
  cacheGet: async () => null,
  cacheSet: () => Promise.resolve(),
  requestXml: async () => noErrorXml,
  ...overrides,
});

test('known no-error response returns no spelling errors', async () => {
  const check = createSpellcheck(createDependencies());

  assert.deepEqual(await check('문장'), []);
});

test('an empty cache entry falls back to the provider', async () => {
  const check = createSpellcheck(
    createDependencies({
      cacheGet: async () => '',
      requestXml: async () => wordXml,
    }),
  );

  const result = await check('문장');

  assert.equal(result.length, 1);
});

test('successful chunks preserve input order and offsets', async () => {
  const check = createSpellcheck(
    createDependencies({
      requestXml: async () => wordXml,
    }),
  );
  const text = `${'a'.repeat(300)}\n${'b'.repeat(300)}`;

  const result = await check(text);

  assert.deepEqual(
    result.map((item) => [item.start, item.context, item.corrections, item.explanation]),
    [
      [0, 'a', [], ''],
      [301, 'b', [], ''],
    ],
  );
});

test('a later chunk failure rejects the whole check', async () => {
  const check = createSpellcheck(
    createDependencies({
      requestXml: async (text) => {
        if (text.startsWith('a')) return wordXml;
        throw new Error('provider failed');
      },
    }),
  );
  const text = `${'a'.repeat(300)}\n${'b'.repeat(300)}`;

  await assert.rejects(() => check(text));
});

test('provider responses are cached only after successful interpretation', async () => {
  const cache = new Map<string, string>();
  let requestCount = 0;
  const check = createSpellcheck({
    cacheGet: async (key) => cache.get(key) ?? null,
    cacheSet: async (key, _ttlSeconds, value) => {
      cache.set(key, value);
    },
    requestXml: async () => (++requestCount === 1 ? providerErrorXml : noErrorXml),
  });

  await assert.rejects(() => check('문장'));
  assert.deepEqual(await check('문장'), []);
  assert.deepEqual(await check('문장'), []);
  assert.equal(requestCount, 2);
});

test('request abort preserves the exact signal reason', async () => {
  const controller = new AbortController();
  const reason = { type: 'request_aborted' };
  const pending = createSpellcheck(
    createDependencies({
      requestXml: (_text, signal) =>
        new Promise<string>((_resolve, reject) => {
          signal?.addEventListener('abort', () => reject(signal.reason), { once: true });
        }),
    }),
  )('문장', controller.signal);

  controller.abort(reason);

  await assert.rejects(
    () => pending,
    (error: unknown) => error === reason,
  );
});

test('chunk failures preserve the source error as diagnostic context', async () => {
  const sourceError = new Error('provider failed');
  const check = createSpellcheck(
    createDependencies({
      requestXml: async () => {
        throw sourceError;
      },
    }),
  );

  await assert.rejects(
    () => check('문장'),
    (error: unknown) => {
      assert.ok(error instanceof Error);
      assert.equal(error.cause, sourceError);
      return true;
    },
  );
});
