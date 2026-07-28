import { describe, expect, it } from 'vitest';
import { canPublish, preparingPage, proofSatisfies, satisfiesWaiter } from './publication';
import type { PublicationTarget } from './publication';

describe('editor publication', () => {
  it('carries an unresolved requirement across a newer no-render revision', () => {
    const target = {
      key: 1,
      requiredRevision: 10,
      proof: { revision: 10, surfaceKey: 1, frameKey: 7 },
      available: true,
    };

    expect(proofSatisfies(target)).toBe(true);
    expect(canPublish(11, 9, { targets: new Map([[0, target]]) })).toBe(true);
  });

  it('requires every current target before publishing atomically', () => {
    const ready: PublicationTarget = {
      key: 1,
      requiredRevision: 10,
      proof: { revision: 10, surfaceKey: 1, frameKey: 7 },
      available: true,
    };
    const waiting: PublicationTarget = {
      key: 2,
      requiredRevision: 10,
      proof: undefined,
      available: true,
    };

    expect(
      canPublish(10, 9, {
        targets: new Map([
          [0, ready],
          [1, waiting],
        ]),
      }),
    ).toBe(false);
  });

  it('rejects a proof delivered to a replaced surface', () => {
    const target = {
      key: 2,
      requiredRevision: 10,
      proof: { revision: 10, surfaceKey: 1, frameKey: 7 },
      available: true,
    };

    expect(proofSatisfies(target)).toBe(false);
    expect(canPublish(10, 9, { targets: new Map([[0, target]]) })).toBe(false);
  });

  it('distinguishes no visual host from an active host with zero targets', () => {
    expect(canPublish(10, 9, undefined)).toBe(false);
    expect(canPublish(10, 9, { targets: new Map() })).toBe(true);
  });

  it('keeps a prior framed publication when the active host temporarily has zero targets', () => {
    expect(canPublish(10, 9, { targets: new Map() }, true, true)).toBe(false);
    expect(canPublish(10, undefined, { targets: new Map() }, true, false)).toBe(true);
  });

  it('lets an ordinary waiter use a retained framed publication when no targets remain', () => {
    expect(satisfiesWaiter(10, 10, new Map([[0, { surfaceKey: 1 }]]), { targets: new Map() })).toBe(true);
  });

  it('keeps frame-required waiters pending without a matching current target', () => {
    expect(satisfiesWaiter(10, 10, new Map([[0, { surfaceKey: 1 }]]), { targets: new Map() }, true)).toBe(false);
  });

  it('requires an exact match while the current target set is non-empty', () => {
    const replacement: PublicationTarget = {
      key: 2,
      requiredRevision: 10,
      proof: undefined,
      available: true,
    };

    expect(satisfiesWaiter(10, 10, new Map([[0, { surfaceKey: 1 }]]), { targets: new Map([[0, replacement]]) })).toBe(false);
  });

  it('requests page zero only when a newer applied layout strands a framed publication without targets', () => {
    expect(preparingPage(true, true, 10, 9, 1, 0)).toBe(0);
    expect(preparingPage(true, false, 10, 9, 1, 0)).toBeUndefined();
    expect(preparingPage(true, true, 9, 9, 1, 0)).toBeUndefined();
    expect(preparingPage(true, true, 10, 9, 0, 0)).toBeUndefined();
    expect(preparingPage(true, true, 10, 9, 1, 1)).toBeUndefined();
    expect(preparingPage(false, true, 10, 9, 1, 0)).toBeUndefined();
  });

  it('republishes a replacement target at the same revision but not a no-change reevaluation', () => {
    const replacement: PublicationTarget = {
      key: 2,
      requiredRevision: 10,
      proof: { revision: 10, surfaceKey: 2, frameKey: 8 },
      available: true,
    };
    const settled: PublicationTarget = { ...replacement, requiredRevision: undefined };

    expect(canPublish(10, 10, { targets: new Map([[0, replacement]]) })).toBe(true);
    expect(canPublish(10, 10, { targets: new Map([[0, settled]]) })).toBe(false);
  });

  it('republishes a same-revision non-empty target shrink only after remaining requirements are ready', () => {
    const ready: PublicationTarget = {
      key: 1,
      requiredRevision: 10,
      proof: { revision: 10, surfaceKey: 1, frameKey: 7 },
      available: true,
    };
    const waiting: PublicationTarget = {
      key: 2,
      requiredRevision: 10,
      proof: undefined,
      available: true,
    };

    expect(canPublish(10, 10, { targets: new Map([[0, ready]]) }, true, true)).toBe(true);
    expect(canPublish(10, 10, { targets: new Map([[1, waiting]]) }, true, true)).toBe(false);
  });
});
