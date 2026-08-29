import { Marked } from 'marked';
import { describe, expect, it } from 'vitest';
import { markedDirectives } from './directives';

const marked = new Marked({ gfm: true, breaks: true }, markedDirectives());
const render = (source: string) => marked.parse(source) as string;

describe('markedDirectives html renderer', () => {
  it('renders a details container with a chevron and a body wrapper', () => {
    const html = render('::: details 버그 수정\n내용\n:::');

    expect(html).toContain('<details><summary>');
    expect(html).toContain('버그 수정</summary>');
    expect(html).toContain('<div data-details-body>');
    expect(html).toContain('<path d="m9 18 6-6-6-6"/>');
  });

  it('escapes the details label', () => {
    expect(render('::: details <b>굵게</b>\n내용\n:::')).toContain('&lt;b&gt;굵게&lt;/b&gt;</summary>');
  });

  it('renders a note container without inline styling', () => {
    const html = render('::: note\n주석\n:::');

    expect(html).toContain('<div data-note>');
    expect(html).not.toContain('style=');
  });

  it('renders a space directive as a sized block', () => {
    expect(render('::: space')).toContain('<div data-space style="height:1.75em"></div>');
    expect(render('::: space 3')).toContain('<div data-space style="height:5.25em"></div>');
  });
});
