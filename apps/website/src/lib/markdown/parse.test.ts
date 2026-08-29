import { describe, expect, it } from 'vitest';
import { parseMarkdown } from './parse';

describe('parseMarkdown', () => {
  it('returns an empty array for empty input', () => {
    expect(parseMarkdown('')).toEqual([]);
  });

  it('parses a paragraph as a single text node', () => {
    expect(parseMarkdown('안녕하세요 반갑습니다')).toEqual([
      { kind: 'paragraph', children: [{ kind: 'text', text: '안녕하세요 반갑습니다' }] },
    ]);
  });

  it('parses headings with their depth', () => {
    expect(parseMarkdown('## 제목')).toEqual([{ kind: 'heading', depth: 2, children: [{ kind: 'text', text: '제목' }] }]);
  });

  it('parses strong, em, del and codespan', () => {
    expect(parseMarkdown('**굵게** *기울임* ~~취소~~ `코드`')).toEqual([
      {
        kind: 'paragraph',
        children: [
          { kind: 'strong', children: [{ kind: 'text', text: '굵게' }] },
          { kind: 'text', text: ' ' },
          { kind: 'em', children: [{ kind: 'text', text: '기울임' }] },
          { kind: 'text', text: ' ' },
          { kind: 'del', children: [{ kind: 'text', text: '취소' }] },
          { kind: 'text', text: ' ' },
          { kind: 'codespan', text: '코드' },
        ],
      },
    ]);
  });

  it('keeps emphasis when a CJK closing quote precedes the delimiter', () => {
    expect(parseMarkdown("**'인용'**을")).toEqual([
      {
        kind: 'paragraph',
        children: [
          { kind: 'strong', children: [{ kind: 'text', text: "'인용'" }] },
          { kind: 'text', text: '을' },
        ],
      },
    ]);
  });

  it('parses links and images', () => {
    expect(parseMarkdown('[타이피](https://typie.co)')).toEqual([
      {
        kind: 'paragraph',
        children: [{ kind: 'link', href: 'https://typie.co', children: [{ kind: 'text', text: '타이피' }] }],
      },
    ]);

    expect(parseMarkdown('![대체 텍스트](https://example.com/a.png)')).toEqual([
      { kind: 'paragraph', children: [{ kind: 'image', src: 'https://example.com/a.png', alt: '대체 텍스트' }] },
    ]);
  });

  it('parses an unordered list', () => {
    expect(parseMarkdown('- 하나\n- 둘')).toEqual([
      {
        kind: 'list',
        ordered: false,
        startIndex: 1,
        items: [
          { task: false, checked: false, blocks: [{ kind: 'paragraph', children: [{ kind: 'text', text: '하나' }] }] },
          { task: false, checked: false, blocks: [{ kind: 'paragraph', children: [{ kind: 'text', text: '둘' }] }] },
        ],
      },
    ]);
  });

  it('parses an ordered list with its start index', () => {
    const blocks = parseMarkdown('3. 셋\n4. 넷');
    expect(blocks).toHaveLength(1);
    expect(blocks[0]).toMatchObject({ kind: 'list', ordered: true, startIndex: 3 });
  });

  it('parses a task list', () => {
    const blocks = parseMarkdown('- [x] 완료\n- [ ] 미완료');
    expect(blocks[0]).toMatchObject({
      kind: 'list',
      items: [
        { task: true, checked: true },
        { task: true, checked: false },
      ],
    });
  });

  it('parses nested list items as nested blocks', () => {
    const blocks = parseMarkdown('- 바깥\n  - 안쪽');
    expect(blocks[0]).toMatchObject({ kind: 'list', ordered: false });
    const outer = blocks[0] as Extract<ReturnType<typeof parseMarkdown>[number], { kind: 'list' }>;
    expect(outer.items[0].blocks.some((block) => block.kind === 'list')).toBe(true);
  });

  it('parses a table with alignments', () => {
    const blocks = parseMarkdown('| 왼쪽 | 가운데 |\n| :--- | :----: |\n| 가 | 나 |');
    expect(blocks[0]).toMatchObject({
      kind: 'table',
      header: { cells: [{ align: 'left' }, { align: 'center' }] },
      rows: [{ cells: [{ align: 'left' }, { align: 'center' }] }],
    });
  });

  it('parses a fenced code block as text', () => {
    expect(parseMarkdown('```\nconst a = 1;\n```')).toEqual([{ kind: 'code', text: 'const a = 1;' }]);
  });

  it('does not treat a typie fence as anything special', () => {
    const blocks = parseMarkdown('```typie:card\n내용\n```');
    expect(blocks).toEqual([{ kind: 'code', text: '내용' }]);
  });

  it('parses a blockquote as nested blocks', () => {
    expect(parseMarkdown('> 인용문')).toEqual([
      { kind: 'blockquote', children: [{ kind: 'paragraph', children: [{ kind: 'text', text: '인용문' }] }] },
    ]);
  });

  it('parses a horizontal rule', () => {
    expect(parseMarkdown('---')).toEqual([{ kind: 'hr' }]);
  });

  it('turns a single newline into a line break', () => {
    expect(parseMarkdown('첫 줄\n둘째 줄')).toEqual([
      {
        kind: 'paragraph',
        children: [{ kind: 'text', text: '첫 줄' }, { kind: 'br' }, { kind: 'text', text: '둘째 줄' }],
      },
    ]);
  });

  it('keeps a raw HTML block as literal text', () => {
    expect(parseMarkdown('<script>alert(1)</script>')).toEqual([
      { kind: 'paragraph', children: [{ kind: 'text', text: '<script>alert(1)</script>' }] },
    ]);
  });

  it('keeps inline HTML tags as literal text', () => {
    expect(parseMarkdown('a <b>bold</b> c')).toEqual([
      {
        kind: 'paragraph',
        children: [
          { kind: 'text', text: 'a ' },
          { kind: 'text', text: '<b>' },
          { kind: 'text', text: 'bold' },
          { kind: 'text', text: '</b>' },
          { kind: 'text', text: ' c' },
        ],
      },
    ]);
  });
});
