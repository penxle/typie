import { describe, expect, it } from 'vitest';
import { attachmentStage, isEmptyLikeStage } from './attachment-stage';

const stage = (overrides: Partial<Parameters<typeof attachmentStage>[0]> = {}) =>
  attachmentStage({ hasAsset: false, localPhase: undefined, hasId: false, unresolved: false, ...overrides });

describe('attachmentStage', () => {
  it('prefers the completed asset over every local and server state', () => {
    expect(stage({ hasAsset: true, localPhase: 'failed', hasId: true, unresolved: true })).toBe('ready');
  });

  it('reports the local attempt while it runs and after it fails', () => {
    expect(stage({ localPhase: 'active', hasId: true })).toBe('localActive');
    expect(stage({ localPhase: 'failed', hasId: true })).toBe('localFailed');
  });

  it('keeps a node the server has not settled yet on the pending card', () => {
    expect(stage({ hasId: true })).toBe('serverPending');
  });

  it('gives an ID-bearing node the server reports as missing the empty placeholder affordance', () => {
    expect(stage({ hasId: true, unresolved: true })).toBe('unresolved');
    expect(isEmptyLikeStage('unresolved')).toBe(true);
    expect(isEmptyLikeStage('empty')).toBe(true);
  });

  it('withholds that affordance from every state that is not settled as absent', () => {
    expect(stage()).toBe('empty');
    expect(isEmptyLikeStage('serverPending')).toBe(false);
    expect(isEmptyLikeStage('localFailed')).toBe(false);
    expect(isEmptyLikeStage('localActive')).toBe(false);
    expect(isEmptyLikeStage('ready')).toBe(false);
  });
});
