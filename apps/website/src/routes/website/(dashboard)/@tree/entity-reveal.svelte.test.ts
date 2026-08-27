import { EntityState } from '@typie/lib/enums';
import { tick } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import {
  consumeEntityTreeRevealRequest,
  createEntityTreeRevealRequest,
  entityTreeRevealState,
  resolveActiveTreeAncestorIds,
  shouldConsumeDocumentRevealRequest,
  shouldOpenEntityTreeFolder,
} from './entity-reveal.svelte';

let destroyEffect: (() => void) | undefined;

afterEach(() => {
  destroyEffect?.();
  destroyEffect = undefined;
  entityTreeRevealState.set(undefined);
});

describe('entity tree reveal', () => {
  it('uses ancestors only while the focused entity belongs to the active tree', () => {
    expect(resolveActiveTreeAncestorIds(EntityState.ACTIVE, ['A', 'B'])).toEqual(['A', 'B']);
    expect(resolveActiveTreeAncestorIds(EntityState.DELETED, ['A', 'B'])).toEqual([]);
  });

  it('snapshots the target path and distinguishes ancestors from the target', () => {
    const source = ['A', 'B'];
    const request = createEntityTreeRevealRequest('NEW', source, true);

    source.push('C');

    expect(request).toEqual({ targetEntityId: 'NEW', ancestorFolderIds: ['A', 'B'], rename: true });
    entityTreeRevealState.set(request);
    expect(shouldOpenEntityTreeFolder('A')).toBe(true);
    expect(shouldOpenEntityTreeFolder('NEW')).toBe(false);
    entityTreeRevealState.set(undefined);
    expect(shouldOpenEntityTreeFolder('A')).toBe(false);
    expect(shouldConsumeDocumentRevealRequest(request, 'NEW', false, true)).toBe(false);
    expect(shouldConsumeDocumentRevealRequest(request, 'NEW', true, false)).toBe(false);
    expect(shouldConsumeDocumentRevealRequest(request, 'NEW', true, true)).toBe(true);
    expect(shouldConsumeDocumentRevealRequest(request, 'OTHER', true, true)).toBe(false);
  });

  it('reopens a collapsed active folder only when the reveal path contains it', async () => {
    let open = $state(false);
    const active = true;

    destroyEffect = $effect.root(() => {
      $effect.pre(() => {
        if (active) {
          open = true;
        }
      });
      $effect.pre(() => {
        if (shouldOpenEntityTreeFolder('A')) {
          open = true;
        }
      });
    });

    await tick();
    expect(open).toBe(true);

    open = false;
    await tick();
    entityTreeRevealState.set(createEntityTreeRevealRequest('NEW', ['B'], true));
    await tick();
    expect(open).toBe(false);

    entityTreeRevealState.set(createEntityTreeRevealRequest('NEW', ['A'], true));
    await tick();

    expect(open).toBe(true);
  });

  it('does not let an older reveal effect consume a newer request', () => {
    const handled = createEntityTreeRevealRequest('OLD', ['A'], false);
    const newer = createEntityTreeRevealRequest('NEW', [], false);

    expect(consumeEntityTreeRevealRequest(newer, handled)).toBe(newer);
    expect(consumeEntityTreeRevealRequest(handled, handled)).toBeUndefined();

    const newerForSameTarget = createEntityTreeRevealRequest('OLD', [], true);
    entityTreeRevealState.set(newerForSameTarget);
    const current = entityTreeRevealState.current;
    expect(current).toEqual(newerForSameTarget);

    entityTreeRevealState.consume(handled);
    expect(entityTreeRevealState.current).toBe(current);

    if (!current) throw new Error('expected a current reveal request');
    entityTreeRevealState.consume(current);
    expect(entityTreeRevealState.current).toBeUndefined();
  });
});
