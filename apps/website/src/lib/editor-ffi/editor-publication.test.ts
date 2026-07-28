import { describe, expect, it } from 'vitest';
import { canPublish, proofSatisfies } from './publication';
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
