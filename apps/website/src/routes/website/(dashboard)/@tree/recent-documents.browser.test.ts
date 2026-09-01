import { afterEach, describe, expect, it } from 'vitest';
import { getNextSiblingOrder, resolveEntityTreeDropTarget } from './utils';

afterEach(() => {
  document.body.replaceChildren();
});

describe('recent document tree boundaries', () => {
  it('resolves structural siblings only inside the authoritative entity tree', () => {
    const recent = document.createElement('div');
    recent.innerHTML = '<a data-id="DOC" data-order="RECENT"></a><a data-id="OTHER" data-order="WRONG"></a>';

    const tree = document.createElement('div');
    tree.dataset.entityTree = 'true';
    tree.innerHTML = '<a data-id="DOC" data-order="100"></a><a data-id="NEXT" data-order="200"></a>';
    document.body.append(recent, tree);

    expect(getNextSiblingOrder('DOC', tree)).toBe('200');
  });

  it('does not resolve recent documents as entity tree drop targets', () => {
    const recent = document.createElement('section');
    recent.innerHTML = '<a data-id="RECENT"><span>Recent document</span></a>';

    const tree = document.createElement('div');
    tree.dataset.entityTree = 'true';
    tree.setAttribute('role', 'tree');
    tree.innerHTML = '<a data-id="TREE"><span>Tree document</span></a>';
    document.body.append(recent, tree);

    expect(resolveEntityTreeDropTarget(tree, recent.querySelector('span'))).toBeNull();
    expect(resolveEntityTreeDropTarget(tree, tree.querySelector('span'))?.dataset.id).toBe('TREE');
  });

  it('resolves blank space in the entity tree surface to the last root entity', () => {
    const tree = document.createElement('div');
    tree.dataset.entityTree = 'true';
    tree.setAttribute('role', 'tree');
    tree.innerHTML = `
      <div>
        <a data-id="FIRST"></a>
        <a data-id="LAST"></a>
      </div>
    `;
    document.body.append(tree);

    expect(resolveEntityTreeDropTarget(tree, tree)?.dataset.id).toBe('LAST');
  });
});
