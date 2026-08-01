import { describe, expect, it } from 'vitest';
import { buildEntityTree } from './admin-entity-tree';

describe('buildEntityTree', () => {
  it('부모-자식 관계로 계층을 만든다', () => {
    const tree = buildEntityTree([
      { id: 'E01', parent: null },
      { id: 'E02', parent: { id: 'E01' } },
      { id: 'E03', parent: { id: 'E02' } },
    ]);

    expect(tree).toHaveLength(1);
    expect(tree[0].entity.id).toBe('E01');
    expect(tree[0].depth).toBe(0);
    expect(tree[0].children.at(0)?.entity.id).toBe('E02');
    expect(tree[0].children.at(0)?.depth).toBe(1);
    expect(tree[0].children.at(0)?.children.at(0)?.entity.id).toBe('E03');
    expect(tree[0].children.at(0)?.children.at(0)?.depth).toBe(2);
  });

  it('입력 순서를 형제 순서로 보존한다', () => {
    const tree = buildEntityTree([
      { id: 'E01', parent: null },
      { id: 'E03', parent: { id: 'E01' } },
      { id: 'E02', parent: { id: 'E01' } },
    ]);

    expect(tree[0].children.map((node) => node.entity.id)).toEqual(['E03', 'E02']);
  });

  it('부모가 목록에 없으면 루트로 올린다', () => {
    const tree = buildEntityTree([{ id: 'E02', parent: { id: 'E99' } }]);

    expect(tree).toHaveLength(1);
    expect(tree[0].entity.id).toBe('E02');
    expect(tree[0].depth).toBe(0);
  });

  it('빈 입력은 빈 배열이다', () => {
    expect(buildEntityTree([])).toEqual([]);
  });

  it('부모가 자식보다 입력 배열에서 뒤에 와도 계층을 만든다', () => {
    const tree = buildEntityTree([
      { id: 'E03', parent: { id: 'E02' } },
      { id: 'E02', parent: { id: 'E01' } },
      { id: 'E01', parent: null },
    ]);

    expect(tree).toHaveLength(1);
    expect(tree[0].entity.id).toBe('E01');
    expect(tree[0].depth).toBe(0);
    expect(tree[0].children.at(0)?.entity.id).toBe('E02');
    expect(tree[0].children.at(0)?.depth).toBe(1);
    expect(tree[0].children.at(0)?.children.at(0)?.entity.id).toBe('E03');
    expect(tree[0].children.at(0)?.children.at(0)?.depth).toBe(2);
  });
});
