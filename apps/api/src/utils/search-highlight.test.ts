import assert from 'node:assert/strict';
import test from 'node:test';
import { sanitizeHighlight } from './search-highlight.ts';

test('highlight keeps bare em tags only and strips every attribute and other tag', () => {
  assert.equal(sanitizeHighlight('a <em>b</em> c'), 'a <em>b</em> c');
  assert.equal(sanitizeHighlight('<em style="position:fixed" class="x" id="y" title="t" onclick="alert(1)">b</em>'), '<em>b</em>');
  assert.equal(sanitizeHighlight('<img src=x onerror="alert(1)"><script>alert(1)</script><a href="javascript:x">l</a>'), 'l');
  assert.equal(sanitizeHighlight('&lt;em style="x"&gt;b&lt;/em&gt;'), '&lt;em style="x"&gt;b&lt;/em&gt;');
  assert.equal(sanitizeHighlight(undefined), undefined);
  assert.equal(sanitizeHighlight(''), undefined);
});
