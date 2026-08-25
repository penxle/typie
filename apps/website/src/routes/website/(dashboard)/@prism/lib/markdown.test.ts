import { describe, expect, it } from 'vitest';
import { clampStreamingTail, parseMarkdown } from './markdown.ts';
import type { BlockNode, InlineNode } from './markdown.ts';

const wordsOf = (nodes: InlineNode[]): string[] =>
  nodes.flatMap((node) => ('children' in node ? wordsOf(node.children) : node.kind === 'word' ? [node.text] : []));

describe('parseMarkdown', () => {
  it('문단·강조·링크를 오프셋 키가 붙은 트리로 만든다', () => {
    const source = '첫 문단이에요\n\n둘째 **굵게** 문단과 [링크](https://x.co)';
    const blocks = parseMarkdown(source);
    expect(blocks.map((block) => block.kind)).toEqual(['paragraph', 'paragraph']);
    expect(blocks[0].key).toBe(0);
    expect(blocks[1].key).toBe(source.indexOf('둘째'));

    const second = blocks[1] as BlockNode & { kind: 'paragraph' };
    const strong = second.children.find((node) => node.kind === 'strong');
    const link = second.children.find((node) => node.kind === 'link');
    expect(strong && wordsOf([strong])).toEqual(['굵게']);
    expect(link).toMatchObject({ href: 'https://x.co' });
  });

  it('단어와 공백을 가르고 각 단어의 key는 원문 오프셋이다', () => {
    const [block] = parseMarkdown('하늘 바다') as (BlockNode & { kind: 'paragraph' })[];
    expect(block.children).toEqual([
      { kind: 'word', key: 0, text: '하늘' },
      { kind: 'space', key: 2, text: ' ' },
      { kind: 'word', key: 3, text: '바다' },
    ]);
  });

  it('접두 파싱과 전문 파싱의 앞부분 키가 일치한다 — 스트리밍 중 스팬이 재마운트되지 않는 근거', () => {
    const source = '첫 문단\n\n- 하나\n- 둘\n\n꼬리 **굵게** 끝';
    const full = parseMarkdown(source);
    for (const cut of [4, 12, 20, source.length - 3]) {
      const partial = parseMarkdown(source.slice(0, cut));
      for (const [index, block] of partial.entries()) {
        if (index < partial.length - 1) expect(block.key).toBe(full[index].key);
      }
    }
  });

  it('미완 구문(굵게·펜스)은 파싱이 깨지지 않고 평문/코드로 선다', () => {
    const [paragraph] = parseMarkdown('여기 **굵') as (BlockNode & { kind: 'paragraph' })[];
    expect(wordsOf(paragraph.children)).toEqual(['여기', '**굵']);

    const blocks = parseMarkdown('코드:\n```js\nconst a');
    expect(blocks[1]).toMatchObject({ kind: 'code', lang: 'js', text: 'const a' });
  });

  it('리스트는 항목별 블록 재귀와 ordered 시작 번호를 갖는다', () => {
    const [list] = parseMarkdown('3. 하나\n4. 둘') as (BlockNode & { kind: 'list' })[];
    expect(list).toMatchObject({ kind: 'list', ordered: true, startIndex: 3 });
    expect(list.items).toHaveLength(2);
    expect(list.items[1].key).toBeGreaterThan(list.items[0].key);
  });

  it('인용·헤딩·수평선을 렌더 트리로 만든다', () => {
    const blocks = parseMarkdown('## 제목\n\n> 인용문\n\n---');
    expect(blocks.map((block) => block.kind)).toEqual(['heading', 'blockquote', 'hr']);
    const quote = blocks[1] as BlockNode & { kind: 'blockquote' };
    expect(quote.children.map((child) => child.kind)).toEqual(['paragraph']);
  });

  it('표는 정렬을 실은 행·칸 트리가 된다', () => {
    const [table] = parseMarkdown('| a | b | c |\n| :- | :-: | -: |\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |') as (BlockNode & { kind: 'table' })[];
    expect(table.kind).toBe('table');
    expect(wordsOf(table.header.cells.flatMap((cell) => cell.children))).toEqual(['a', 'b', 'c']);
    expect(table.header.cells.map((cell) => cell.align)).toEqual(['left', 'center', 'right']);
    expect(table.rows).toHaveLength(2);
    expect(wordsOf(table.rows[1].cells.flatMap((cell) => cell.children))).toEqual(['4', '5', '6']);

    const keys = [table.header, ...table.rows].flatMap((row) => [row.key, ...row.cells.map((cell) => cell.key)]);
    expect(new Set(keys).size).toBe(keys.length);
  });

  it('할 일 목록은 체크 상태를 항목에 싣고 표식 텍스트를 남기지 않는다', () => {
    const [list] = parseMarkdown('- [ ] 미완\n- [x] 완료\n- 보통') as (BlockNode & { kind: 'list' })[];
    expect(list.items.map((item) => [item.task, item.checked])).toEqual([
      [true, false],
      [true, true],
      [false, false],
    ]);
    expect(list.items.map((item) => item.blocks.length)).toEqual([1, 1, 1]);
    expect(list.items.flatMap((item) => wordsOf((item.blocks[0] as BlockNode & { kind: 'paragraph' }).children))).toEqual([
      '미완',
      '완료',
      '보통',
    ]);
  });

  it('이미지는 인라인 노드가 되고 참조 정의는 렌더 트리에서 사라진다', () => {
    const [paragraph] = parseMarkdown('앞 ![대체](https://x.co/a.png) 뒤') as (BlockNode & { kind: 'paragraph' })[];
    expect(paragraph.children.find((node) => node.kind === 'image')).toMatchObject({ src: 'https://x.co/a.png', alt: '대체' });

    const blocks = parseMarkdown('[참조][ref] 문장\n\n[ref]: https://x.co');
    expect(blocks.map((block) => block.kind)).toEqual(['paragraph']);
    const [linked] = blocks as (BlockNode & { kind: 'paragraph' })[];
    expect(linked.children.find((node) => node.kind === 'link')).toMatchObject({ href: 'https://x.co' });
  });

  it('여전히 미지원인 토큰은 원문 그대로 평문 폴백한다', () => {
    const [html] = parseMarkdown('<div>블록</div>');
    expect(html.kind).toBe('paragraph');
  });

  it('typie: lang 펜스는 카드 노드로 승격된다', () => {
    const source = '앞 문장.\n\n```typie:document\n{"id": "D1"}\n```';
    const blocks = parseMarkdown(source);
    expect(blocks[1]).toMatchObject({ kind: 'card', name: 'document', text: '{"id": "D1"}', pending: false });
    expect(blocks[1].key).toBe(source.indexOf('```'));
  });

  it('미폐쇄 typie: 펜스는 pending 카드다', () => {
    expect(parseMarkdown('```typie:document\n{"id')[0]).toMatchObject({ kind: 'card', name: 'document', pending: true });
    expect(parseMarkdown('```typie:document\n')[0]).toMatchObject({ kind: 'card', name: 'document', text: '', pending: true });
  });

  it('일반 lang 펜스는 카드로 승격되지 않는다', () => {
    expect(parseMarkdown('```js\nconst a = 1\n```')[0]).toMatchObject({ kind: 'code', lang: 'js', text: 'const a = 1' });
  });

  it('카드 문법을 벗어난 변형은 코드 블록으로 남는다', () => {
    expect(parseMarkdown('~~~typie:document\n{"id": "D1"}\n~~~')[0]).toMatchObject({ kind: 'code' });
    expect(parseMarkdown('```typie:document meta\n{}\n```')[0]).toMatchObject({ kind: 'code' });
    expect(parseMarkdown('```typie:Document\n{}\n```')[0]).toMatchObject({ kind: 'code' });
    expect(parseMarkdown('```typie:\n{}\n```')[0]).toMatchObject({ kind: 'code' });
  });
});

describe('clampStreamingTail', () => {
  it('카드 펜스 열림줄의 접두인 꼬리줄을 제외한다', () => {
    expect(clampStreamingTail('문장.\n\n``')).toBe('문장.\n');
    expect(clampStreamingTail('문장.\n\n```typie:doc')).toBe('문장.\n');
    expect(clampStreamingTail('```typie:document')).toBe('');
  });

  it('일반 펜스·문중 백틱·평문은 건드리지 않는다', () => {
    expect(clampStreamingTail('```python')).toBe('```python');
    expect(clampStreamingTail('앞에 `코드` 뒤')).toBe('앞에 `코드` 뒤');
    expect(clampStreamingTail('문장 그대로')).toBe('문장 그대로');
  });

  it('열림줄이 개행으로 완성되면 더는 클램프하지 않는다', () => {
    expect(clampStreamingTail('```typie:document\n')).toBe('```typie:document\n');
  });

  it('카드 닫힘줄이 미완이어도 pending 카드로 선다', () => {
    const clamped = clampStreamingTail('```typie:document\n{"id": "D1"}\n``');
    expect(parseMarkdown(clamped)[0]).toMatchObject({ kind: 'card', pending: true });
  });

  it('개행 없이 닫힌 카드 펜스는 클램프하지 않는다 — 완성 카드가 pending으로 후퇴하지 않는다', () => {
    const text = '```typie:document\n{"id": "D1"}\n```';
    expect(clampStreamingTail(text)).toBe(text);
    expect(parseMarkdown(text)[0]).toMatchObject({ kind: 'card', pending: false });
  });

  it('열린 카드 펜스 안의 백틱 꼬리줄은 클램프하지 않는다', () => {
    const text = '```typie:document\n{"id": "D1"}\n``';
    expect(clampStreamingTail(text)).toBe(text);
    expect(parseMarkdown(text)[0]).toMatchObject({ kind: 'card', pending: true });
  });

  it('카드가 닫힌 뒤의 백틱 꼬리줄은 다시 클램프한다', () => {
    expect(clampStreamingTail('```typie:document\n{"id": "D1"}\n```\n\n``')).toBe('```typie:document\n{"id": "D1"}\n```\n');
  });

  it('틸드 펜스는 카드 펜스로 취급되지 않는다 — 열림 가드가 켜지지 않는다', () => {
    expect(clampStreamingTail('~~~typie:document\n{"id": "D1"}\n``')).toBe('~~~typie:document\n{"id": "D1"}');
  });
});
