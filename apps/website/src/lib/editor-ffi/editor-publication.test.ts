import { describe, expect, it } from 'vitest';
import { proofSatisfies, satisfiesWaiter } from './publication';
import type { PublicationTarget } from './publication';

describe('editor publication', () => {
  it('accepts only a proof for the current surface and required revision', () => {
    const target: PublicationTarget = {
      key: 2,
      requiredRevision: 10,
      proof: { revision: 10, surfaceKey: 2, frameKey: 7 },
      available: true,
    };
    const proof = target.proof;
    if (!proof) throw new Error('TEST HARNESS: target proof is missing');

    expect(proofSatisfies(target)).toBe(true);
    expect(proofSatisfies({ ...target, proof: { ...proof, surfaceKey: 1 } })).toBe(false);
    expect(proofSatisfies({ ...target, proof: { ...proof, revision: 9 } })).toBe(false);
    expect(proofSatisfies({ ...target, available: false })).toBe(false);
  });

  it('keeps an accepted publication valid when the producer cohort changes', () => {
    expect(satisfiesWaiter(10, 10, new Map([[0, { surfaceKey: 1 }]]))).toBe(true);
  });

  it('distinguishes an ordinary empty publication from a frame-required one', () => {
    expect(satisfiesWaiter(10, 10, new Map())).toBe(true);
    expect(satisfiesWaiter(10, 10, new Map(), true)).toBe(false);
  });
});
