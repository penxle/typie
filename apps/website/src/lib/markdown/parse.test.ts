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

  it('parses a details fence with its summary and nested blocks', () => {
    expect(parseMarkdown('::: details 버그 수정\n- 첫번째\n- 두번째\n:::')).toEqual([
      {
        kind: 'details',
        summary: '버그 수정',
        children: [
          {
            kind: 'list',
            ordered: false,
            startIndex: 1,
            items: [
              { task: false, checked: false, blocks: [{ kind: 'paragraph', children: [{ kind: 'text', text: '첫번째' }] }] },
              { task: false, checked: false, blocks: [{ kind: 'paragraph', children: [{ kind: 'text', text: '두번째' }] }] },
            ],
          },
        ],
      },
    ]);
  });

  it('keeps a details fence with no summary', () => {
    expect(parseMarkdown('::: details\n내용\n:::')).toEqual([
      { kind: 'details', summary: '', children: [{ kind: 'paragraph', children: [{ kind: 'text', text: '내용' }] }] },
    ]);
  });

  it('runs an unclosed details fence to the end of the input', () => {
    expect(parseMarkdown('::: details 제목\n내용')).toEqual([
      { kind: 'details', summary: '제목', children: [{ kind: 'paragraph', children: [{ kind: 'text', text: '내용' }] }] },
    ]);
  });

  it('nests a details fence inside a longer one', () => {
    expect(parseMarkdown(':::: details 바깥\n::: details 안쪽\n내용\n:::\n::::')).toEqual([
      {
        kind: 'details',
        summary: '바깥',
        children: [{ kind: 'details', summary: '안쪽', children: [{ kind: 'paragraph', children: [{ kind: 'text', text: '내용' }] }] }],
      },
    ]);
  });

  it('leaves a bare colon fence as text', () => {
    expect(parseMarkdown(':::\n내용\n:::')).toEqual([
      {
        kind: 'paragraph',
        children: [
          { kind: 'text', text: ':::' },
          { kind: 'br' },
          { kind: 'text', text: '내용' },
          { kind: 'br' },
          { kind: 'text', text: ':::' },
        ],
      },
    ]);
  });

  it('parses consecutive details fences independently', () => {
    expect(parseMarkdown('::: details 첫째\n가\n:::\n\n::: details 둘째\n나\n:::')).toEqual([
      { kind: 'details', summary: '첫째', children: [{ kind: 'paragraph', children: [{ kind: 'text', text: '가' }] }] },
      { kind: 'details', summary: '둘째', children: [{ kind: 'paragraph', children: [{ kind: 'text', text: '나' }] }] },
    ]);
  });

  it('parses a space directive with a default of one line', () => {
    expect(parseMarkdown('앞\n\n::: space\n\n뒤')).toEqual([
      { kind: 'paragraph', children: [{ kind: 'text', text: '앞' }] },
      { kind: 'space', lines: 1 },
      { kind: 'paragraph', children: [{ kind: 'text', text: '뒤' }] },
    ]);
  });

  it('parses an explicit line count on a space directive', () => {
    expect(parseMarkdown('::: space 3')).toEqual([{ kind: 'space', lines: 3 }]);
  });

  it('leaves a space directive with a non-numeric argument as text', () => {
    expect(parseMarkdown('::: space 가나')).toEqual([{ kind: 'paragraph', children: [{ kind: 'text', text: '::: space 가나' }] }]);
  });

  it('parses a space directive nested inside a details block', () => {
    expect(parseMarkdown('::: details 제목\n앞\n\n::: space 2\n\n뒤\n:::')).toEqual([
      {
        kind: 'details',
        summary: '제목',
        children: [
          { kind: 'paragraph', children: [{ kind: 'text', text: '앞' }] },
          { kind: 'space', lines: 2 },
          { kind: 'paragraph', children: [{ kind: 'text', text: '뒤' }] },
        ],
      },
    ]);
  });

  it('parses a note container with its nested blocks', () => {
    expect(parseMarkdown('::: note\n작은 글씨\n:::')).toEqual([
      { kind: 'note', children: [{ kind: 'paragraph', children: [{ kind: 'text', text: '작은 글씨' }] }] },
    ]);
  });

  it('leaves a note fence carrying a label as text', () => {
    expect(parseMarkdown('::: note 제목\n내용\n:::')).toEqual([
      {
        kind: 'paragraph',
        children: [
          { kind: 'text', text: '::: note 제목' },
          { kind: 'br' },
          { kind: 'text', text: '내용' },
          { kind: 'br' },
          { kind: 'text', text: ':::' },
        ],
      },
    ]);
  });

  it('nests a note inside a details block', () => {
    expect(parseMarkdown(':::: details 제목\n::: note\n주석\n:::\n::::')).toEqual([
      {
        kind: 'details',
        summary: '제목',
        children: [{ kind: 'note', children: [{ kind: 'paragraph', children: [{ kind: 'text', text: '주석' }] }] }],
      },
    ]);
  });
});
