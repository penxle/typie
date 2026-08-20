import { describe, expect, it } from 'vitest';
import { parseMarkdown } from './markdown.ts';
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

  it('미지원 토큰은 원문 그대로 평문 폴백한다', () => {
    const [table] = parseMarkdown('| a | b |\n| - | - |\n| 1 | 2 |');
    expect(table.kind).toBe('paragraph');
  });
});
