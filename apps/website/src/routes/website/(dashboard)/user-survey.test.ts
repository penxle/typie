import { describe, expect, it } from 'vitest';
import {
  buildUserSurveyValue,
  canAdvanceUserSurvey,
  createUserSurveyDraft,
  orderUserSurveyOptions,
  selectUserSurveyOption,
  USER_SURVEY_QUESTIONS,
  userSurveySnoozeUntil,
  visibleUserSurveyInputs,
} from './user-survey.ts';
import type { UserSurveyQuestion } from './user-survey.ts';

const question = (id: UserSurveyQuestion['id']): UserSurveyQuestion => {
  const found = USER_SURVEY_QUESTIONS.find((candidate) => candidate.id === id);
  if (!found) {
    throw new Error(`question ${id} not found`);
  }
  return found;
};

const answer = (selected: string[], inputs: Record<string, string> = {}, text = '') => ({ selected, inputs, text });

describe('user-survey', () => {
  it('문항 id 와 각 문항의 선택지 value 는 겹치지 않는다', () => {
    const ids = USER_SURVEY_QUESTIONS.map((candidate) => candidate.id);
    expect(new Set(ids).size).toBe(ids.length);

    for (const candidate of USER_SURVEY_QUESTIONS) {
      if (candidate.kind === 'text') {
        continue;
      }
      const values = candidate.options.map((option) => option.value);
      expect(new Set(values).size).toBe(values.length);
    }
  });

  it('고정 선택지는 섞이지 않고 선언 순서대로 맨 아래에 온다', () => {
    const source = question('source');
    if (source.kind === 'text') {
      throw new Error('unexpected');
    }

    const ordered = orderUserSurveyOptions(source, () => 0);
    const pinned = source.options.filter((option) => option.pinned).map((option) => option.value);
    expect(ordered.slice(-pinned.length).map((option) => option.value)).toEqual(pinned);

    const unpinned = source.options.filter((option) => !option.pinned).map((option) => option.value);
    const head = ordered.slice(0, -pinned.length).map((option) => option.value);
    const byValue = (a: string, b: string) => a.localeCompare(b);
    expect([...head].toSorted(byValue)).toEqual([...unpinned].toSorted(byValue));
  });

  it('순서형 문항은 섞지 않고 선언 순서를 지킨다', () => {
    const dependence = question('dependence');
    if (dependence.kind === 'text') {
      throw new Error('unexpected');
    }

    const ordered = orderUserSurveyOptions(dependence, () => 0).map((option) => option.value);
    expect(ordered).toEqual(dependence.options.map((option) => option.value));
  });

  it('단일 선택은 마지막 선택으로 바뀌고, 복수 선택은 토글되며 상한이 없으면 계속 추가된다', () => {
    const single = question('reason');
    expect(selectUserSurveyOption(single, answer(['focus']), 'sync')).toEqual(['sync']);

    const multi = question('genres');
    expect(selectUserSurveyOption(multi, answer([]), 'novel')).toEqual(['novel']);
    expect(selectUserSurveyOption(multi, answer(['novel', 'poem']), 'essay')).toEqual(['novel', 'poem', 'essay']);
    expect(selectUserSurveyOption(multi, answer(['novel', 'poem']), 'poem')).toEqual(['novel']);
  });

  it('복수 선택에 상한이 있으면 그 이상의 추가를 무시한다', () => {
    const capped = { ...question('genres'), max: 2 } as UserSurveyQuestion;
    expect(selectUserSurveyOption(capped, answer(['novel', 'poem']), 'essay')).toEqual(['novel', 'poem']);
  });

  it('고르기 전에는 넘어갈 수 없고 직접 입력은 공백이 아닌 텍스트가 있어야 넘어간다', () => {
    const reason = question('reason');
    expect(canAdvanceUserSurvey(reason, answer([]))).toBe(false);
    expect(canAdvanceUserSurvey(reason, answer(['focus']))).toBe(true);
    expect(canAdvanceUserSurvey(reason, answer(['other']))).toBe(false);
    expect(canAdvanceUserSurvey(reason, answer(['other'], { other: ' '.repeat(3) }))).toBe(false);
    expect(canAdvanceUserSurvey(reason, answer(['other'], { other: '폰트' }))).toBe(true);
  });

  it('선택 입력은 비워도 넘어갈 수 있고 고른 선택지의 입력란만 보인다', () => {
    const source = question('source');
    expect(canAdvanceUserSurvey(source, answer(['community']))).toBe(true);
    expect(visibleUserSurveyInputs(source, answer(['community'])).map((option) => option.value)).toEqual(['community']);
    expect(visibleUserSurveyInputs(source, answer(['search'])).map((option) => option.value)).toEqual(['search']);
    expect(visibleUserSurveyInputs(source, answer(['youtube']))).toEqual([]);
  });

  it('자유 의견은 비워도 완료할 수 있다', () => {
    expect(canAdvanceUserSurvey(question('feedback'), answer([]))).toBe(true);
  });

  it('값은 고른 선택지의 입력만 다듬어 싣고 나머지는 비운다', () => {
    const draft = createUserSurveyDraft();
    draft.genres = answer(['novel', 'other'], { other: ' 설정집 ' });
    draft.source = answer(['search'], { community: '남은 텍스트', other: '남은 텍스트', search: ' 글쓰기 앱 ' });
    draft.previous_tool = answer(['hwp']);
    draft.reason = answer(['other'], { other: ' 루비 문자 ' });
    draft.dependence = answer(['very']);
    draft.feedback = answer([], {}, ' 내보내기가 아쉬워요 ');

    expect(buildUserSurveyValue(draft)).toEqual({
      genres: ['novel', 'other'],
      genres_other: '설정집',
      source: 'search',
      source_other: '',
      source_community: '',
      source_search: '글쓰기 앱',
      previous_tool: 'hwp',
      previous_tool_other: '',
      reason: 'other',
      reason_other: '루비 문자',
      dependence: 'very',
      feedback: '내보내기가 아쉬워요',
    });
  });

  it('스누즈는 30일 뒤로 잡힌다', () => {
    const now = new Date('2026-09-02T00:00:00Z');
    expect(userSurveySnoozeUntil(now).toISOString()).toBe('2026-10-02T00:00:00.000Z');
  });
});
