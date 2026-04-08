import assert from 'node:assert/strict';
import test from 'node:test';
import { HTTPException } from 'hono/http-exception';
import { normalizeSpecimenFallbacks, renderFontSpecimenSvg } from './font-specimen.ts';

test('normalizeSpecimenFallbacks trims blanks and excludes duplicates of the primary text', () => {
  assert.deepEqual(normalizeSpecimenFallbacks('보통', [' ', 'Regular', '보통', 'regular', '400']), ['Regular', '400']);
});

test('renderFontSpecimenSvg retries missing glyphs and applies root color', async () => {
  const attempts: string[] = [];

  const svg = await renderFontSpecimenSvg({
    text: '보통',
    fallbacks: ['Regular', '400'],
    color: '#F1F1F7',
    renderTextToSvg: async (candidate) => {
      attempts.push(candidate);
      if (candidate === '보통') {
        throw new Error('missing glyph');
      }

      return '<svg xmlns="http://www.w3.org/2000/svg"><path fill="currentColor" /></svg>';
    },
  });

  assert.deepEqual(attempts, ['보통', 'Regular']);
  assert.match(svg, /^<svg color="#F1F1F7"/);
});

test('renderFontSpecimenSvg throws 422 when every candidate is missing glyph', async () => {
  await assert.rejects(
    () =>
      renderFontSpecimenSvg({
        text: '보통',
        fallbacks: ['Regular'],
        renderTextToSvg: async () => {
          throw new Error('missing glyph');
        },
      }),
    (err: unknown) => err instanceof HTTPException && err.status === 422,
  );
});
