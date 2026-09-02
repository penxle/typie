import { describe, expect, it } from 'vitest';
import {
  buildCancellationSurveyValue,
  CANCELLATION_REASONS,
  cancellationTextInput,
  canSubmitCancellationSurvey,
  orderCancellationReasons,
} from './cancellation-survey.ts';

describe('cancellation-survey', () => {
  it('선택지는 개념 7개와 고정 2개로 구성되고 value 가 겹치지 않는다', () => {
    const values = CANCELLATION_REASONS.map((option) => option.value);
    expect(new Set(values).size).toBe(values.length);
    expect(CANCELLATION_REASONS.filter((option) => !option.pinned)).toHaveLength(7);
    expect(CANCELLATION_REASONS.filter((option) => option.pinned).map((option) => option.value)).toEqual(['other', 'prefer_not_to_say']);
  });

  it('고정 선택지는 섞이지 않고 선언 순서대로 맨 아래에 온다', () => {
    const reversed = orderCancellationReasons(() => 0);
    const tail = reversed.slice(-2).map((option) => option.value);
    expect(tail).toEqual(['other', 'prefer_not_to_say']);

    const unpinned = CANCELLATION_REASONS.filter((option) => !option.pinned).map((option) => option.value);
    const head = reversed.slice(0, -2).map((option) => option.value);
    const byValue = (a: string, b: string) => a.localeCompare(b);
    expect([...head].toSorted(byValue)).toEqual([...unpinned].toSorted(byValue));
  });

  it('의견란은 이유를 고르기 전에도 선택 입력으로 보이고, 직접 입력만 필수이며, 답하고 싶지 않아요는 숨긴다', () => {
    expect(cancellationTextInput(null)).toBe('optional');
    expect(cancellationTextInput('low_usage')).toBe('optional');
    expect(cancellationTextInput('other')).toBe('required');
    expect(cancellationTextInput('prefer_not_to_say')).toBeNull();
  });

  it('이유를 고르기 전에는 제출할 수 없다', () => {
    expect(canSubmitCancellationSurvey({ reason: null, text: '' })).toBe(false);
    expect(canSubmitCancellationSurvey({ reason: 'low_usage', text: '' })).toBe(true);
  });

  it('직접 입력은 공백이 아닌 텍스트가 있어야 제출할 수 있다', () => {
    expect(canSubmitCancellationSurvey({ reason: 'other', text: '' })).toBe(false);
    expect(canSubmitCancellationSurvey({ reason: 'other', text: ' '.repeat(3) })).toBe(false);
    expect(canSubmitCancellationSurvey({ reason: 'other', text: '내보내기가 없어서' })).toBe(true);
  });

  it('답하고 싶지 않아요는 바로 제출할 수 있다', () => {
    expect(canSubmitCancellationSurvey({ reason: 'prefer_not_to_say', text: '' })).toBe(true);
  });

  it('직접 입력의 텍스트는 reason_other 에만 실리고 detail 은 비운다', () => {
    expect(buildCancellationSurveyValue({ reason: 'other', text: ' 내보내기가 없어서 ' })).toEqual({
      reason: 'other',
      reason_other: '내보내기가 없어서',
      detail: '',
    });
  });

  it('답하고 싶지 않아요는 남아 있던 텍스트를 싣지 않는다', () => {
    expect(buildCancellationSurveyValue({ reason: 'prefer_not_to_say', text: '남은 텍스트' })).toEqual({
      reason: 'prefer_not_to_say',
      reason_other: '',
      detail: '',
    });
  });

  it('개념 선택지는 detail 만 다듬어 싣고 reason_other 는 비운다', () => {
    expect(buildCancellationSurveyValue({ reason: 'unstable', text: ' 긴 글에서 렉 ' })).toEqual({
      reason: 'unstable',
      reason_other: '',
      detail: '긴 글에서 렉',
    });
  });
});
