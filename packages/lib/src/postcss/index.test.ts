import assert from 'node:assert/strict';
import test from 'node:test';
import postcss from 'postcss';
import plugin from './index.js';

const run = async (css: string) => {
  const result = await postcss([plugin()]).process(css, { from: undefined });
  return result.css.replaceAll(/\s+/g, ' ').trim();
};

test('Panda 조건 형태는 자손 전체를 not(light *)로 감싼다', async () => {
  const css = await run('[data-theme="dark"] .a { color: red; }');
  assert.equal(
    css,
    '[data-theme="dark"] .a { color: red; } @media (prefers-color-scheme: dark) { .a:not([data-theme="light"] *) { color: red; } }',
  );
});

test('루트 단독 형태는 :root:not(light)가 된다', async () => {
  const css = await run('[data-theme=dark] { --x: 1; }');
  assert.equal(css, '[data-theme=dark] { --x: 1; } @media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { --x: 1; } }');
});

test(':root[data-theme=dark] 자손 형태는 결합자와 의사 요소를 보존한다', async () => {
  const css = await run(":root[data-theme='dark'] .a::before { color: red; }");
  assert.equal(
    css,
    ':root[data-theme=\'dark\'] .a::before { color: red; } @media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) .a::before { color: red; } }',
  );
});

test(':root[data-theme=dark] 단독 형태는 :root:not(light)가 된다', async () => {
  const css = await run(":root[data-theme='dark'] { --x: 1; }");
  assert.equal(
    css,
    ':root[data-theme=\'dark\'] { --x: 1; } @media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { --x: 1; } }',
  );
});

test('variant 형태는 기존 출력을 유지한다', async () => {
  const css = await run('[data-theme="dark"][data-variant-dark="nord"] .a { color: red; }');
  assert.equal(
    css,
    '[data-theme="dark"][data-variant-dark="nord"] .a { color: red; } @media (prefers-color-scheme: dark) { .a:not([data-theme="light"] *)[data-variant-dark="nord"] { color: red; } }',
  );
});
