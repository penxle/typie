import { describe, expect, it, vi } from 'vitest';
import { labelForRequest } from './tool-labels.ts';

vi.mock('@sentry/sveltekit', () => ({ captureMessage: vi.fn() }));

describe('labelForRequest', () => {
  it('성공·미해소는 도구 라벨, 실패 봉투는 실패 라벨을 고른다', () => {
    expect(labelForRequest('save-document', { ok: true })).toBe('문서를 저장했어요');
    expect(labelForRequest('save-document', undefined)).toBe('문서를 저장했어요');
    expect(labelForRequest('save-document', { ok: false, code: 'error', message: 'x' })).toBe('문서를 저장하지 못했어요');
    expect(labelForRequest('create-documents', { ok: false, code: 'error', message: 'x' })).toBe('처리하지 못했어요');
    expect(labelForRequest('unknown-tool', undefined)).toBeNull();
  });
});
