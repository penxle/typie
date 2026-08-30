import assert from 'node:assert/strict';
import test from 'node:test';
import { extractGatewayHeaders, extractUsage, textIdentity } from './llm-usage-core.ts';

test('textIdentity counts codepoints, not UTF-16 units', () => {
  assert.equal(textIdentity('가나다🙂').textLength, 4);
});

test('textIdentity collapses whitespace before hashing', () => {
  assert.equal(textIdentity('  가나다  \n\n 라마바 ').fullHash, textIdentity('가나다 라마바').fullHash);
});

test('textIdentity gives identical hashes for texts shorter than the prefix window', () => {
  const identity = textIdentity('짧은 원고');
  assert.equal(identity.prefixHash, identity.fullHash);
});

test('textIdentity keeps prefixHash stable when only the tail changes', () => {
  const head = '가'.repeat(1000);
  const a = textIdentity(head + '뒤쪽 원본');
  const b = textIdentity(head + '뒤쪽을 완전히 다르게 고쳤다');

  assert.equal(a.prefixHash, b.prefixHash);
  assert.notEqual(a.fullHash, b.fullHash);
});

test('textIdentity changes prefixHash when the head changes', () => {
  const tail = '나'.repeat(1000);
  const a = textIdentity('앞쪽 원본' + tail);
  const b = textIdentity('앞쪽을 고쳤다' + tail);

  assert.notEqual(a.prefixHash, b.prefixHash);
});

test('textIdentity normalizes before slicing the prefix window', () => {
  const body = '가'.repeat(1200);
  assert.equal(textIdentity(body).prefixHash, textIdentity(`\n\n   ${body}`).prefixHash);
});

test('extractUsage pulls all five token counts', () => {
  const usage = {
    prompt_tokens: 100,
    completion_tokens: 20,
    total_tokens: 128,
    prompt_tokens_details: { cached_tokens: 64 },
    completion_tokens_details: { reasoning_tokens: 8 },
  };

  assert.deepEqual(extractUsage(usage), {
    inputTokens: 100,
    outputTokens: 20,
    cachedInputTokens: 64,
    reasoningTokens: 8,
    totalTokens: 128,
  });
});

test('extractUsage returns nulls when usage is absent', () => {
  assert.deepEqual(extractUsage(null), {
    inputTokens: null,
    outputTokens: null,
    cachedInputTokens: null,
    reasoningTokens: null,
    totalTokens: null,
  });
});

test('extractUsage tolerates missing detail objects', () => {
  const usage = { prompt_tokens: 10, completion_tokens: 2, total_tokens: 12 };

  assert.deepEqual(extractUsage(usage), {
    inputTokens: 10,
    outputTokens: 2,
    cachedInputTokens: null,
    reasoningTokens: null,
    totalTokens: 12,
  });
});

test('extractUsage keeps reasoning tokens separate from completion tokens', () => {
  // Vertex AI 경유 Gemini 실측 형태 — total = prompt + completion + reasoning
  const usage = {
    prompt_tokens: 13,
    completion_tokens: 1,
    total_tokens: 144,
    completion_tokens_details: { reasoning_tokens: 130 },
  };

  const extracted = extractUsage(usage);
  assert.equal(extracted.outputTokens, 1);
  assert.equal(extracted.reasoningTokens, 130);
  assert.ok(extracted.inputTokens !== null && extracted.outputTokens !== null && extracted.reasoningTokens !== null);
  assert.equal(extracted.inputTokens + extracted.outputTokens + extracted.reasoningTokens, extracted.totalTokens);
});

test('extractGatewayHeaders reads the gateway headers', () => {
  const response = new Response(null, {
    headers: { 'cf-aig-cache-status': 'HIT', 'cf-aig-log-id': 'abc123' },
  });

  assert.deepEqual(extractGatewayHeaders(response), { cacheStatus: 'HIT', gatewayLogId: 'abc123' });
});

test('extractGatewayHeaders returns nulls when the headers are absent', () => {
  assert.deepEqual(extractGatewayHeaders(new Response(null)), { cacheStatus: null, gatewayLogId: null });
});
