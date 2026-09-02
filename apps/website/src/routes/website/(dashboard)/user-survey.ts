export const USER_SURVEY_NAME = 'user_research_202609';
export const USER_SURVEY_SNOOZE_KEY = 'surveySkipUntil';
export const USER_SURVEY_SNOOZE_DAYS = 30;

export type UserSurveyQuestionId = 'genres' | 'source' | 'previous_tool' | 'reason' | 'dependence' | 'feedback';

export type UserSurveyOption = {
  value: string;
  label: string;
  input?: 'required' | 'optional';
  prompt?: string;
  pinned?: boolean;
};

type ChoiceQuestion = {
  id: Exclude<UserSurveyQuestionId, 'feedback'>;
  kind: 'single' | 'multi';
  title: string;
  hint: string;
  options: UserSurveyOption[];
  columns: 1 | 2;
  shuffle: boolean;
  max?: number;
};

type TextQuestion = {
  id: 'feedback';
  kind: 'text';
  title: string;
  hint: string;
  placeholder: string;
};

export type UserSurveyQuestion = ChoiceQuestion | TextQuestion;

export const USER_SURVEY_QUESTIONS: UserSurveyQuestion[] = [
  {
    id: 'genres',
    kind: 'multi',
    title: '타이피에서 주로 어떤 글을 쓰시나요?',
    hint: '해당하는 것을 모두 골라 주세요',
    columns: 2,
    shuffle: true,
    options: [
      { value: 'novel', label: '소설·웹소설' },
      { value: 'fanfic', label: '2차창작' },
      { value: 'essay', label: '에세이·일기' },
      { value: 'poem', label: '시' },
      { value: 'script', label: '시나리오·대본' },
      { value: 'blog', label: '블로그·리뷰' },
      { value: 'academic', label: '학업·업무 문서' },
      { value: 'other', label: '직접 입력', input: 'required', prompt: '어떤 글인지 알려주세요', pinned: true },
    ],
  },
  {
    id: 'source',
    kind: 'single',
    title: '타이피를 처음 어떻게 알게 되셨나요?',
    hint: '하나만 골라 주세요',
    columns: 2,
    shuffle: true,
    options: [
      { value: 'x', label: 'X (트위터)' },
      { value: 'instagram', label: '인스타그램·스레드' },
      { value: 'youtube', label: '유튜브' },
      { value: 'search', label: '검색', input: 'optional', prompt: '무엇을 검색하셨나요?' },
      { value: 'friend', label: '친구·지인 추천' },
      { value: 'community', label: '창작자 커뮤니티', input: 'optional', prompt: '어떤 커뮤니티인가요?' },
      { value: 'ad', label: '광고' },
      { value: 'app_store', label: 'App Store·Google Play' },
      { value: 'other', label: '직접 입력', input: 'required', prompt: '어떻게 알게 되셨는지 알려주세요', pinned: true },
      { value: 'unknown', label: '기억나지 않아요', pinned: true },
    ],
  },
  {
    id: 'previous_tool',
    kind: 'single',
    title: '타이피를 쓰기 전, 가장 많이 쓴 도구는 무엇인가요?',
    hint: '하나만 골라 주세요',
    columns: 2,
    shuffle: true,
    options: [
      { value: 'hwp', label: '한글' },
      { value: 'word', label: 'MS Word' },
      { value: 'docs', label: 'Google Docs' },
      { value: 'notion', label: 'Notion' },
      { value: 'memo', label: '기본 메모 앱' },
      { value: 'note_app', label: '노트 앱 (에버노트 등)' },
      { value: 'scrivener', label: 'Scrivener' },
      { value: 'other', label: '직접 입력', input: 'required', prompt: '어떤 도구인지 알려주세요', pinned: true },
    ],
  },
  {
    id: 'reason',
    kind: 'single',
    title: '타이피를 계속 쓰는 가장 큰 이유는 무엇인가요?',
    hint: '하나만 골라 주세요',
    columns: 2,
    shuffle: true,
    options: [
      { value: 'focus', label: '글에만 집중되는 화면' },
      { value: 'sync', label: '자동 저장·기기 간 이어쓰기' },
      { value: 'formatting', label: '서식과 폰트 조정' },
      { value: 'spellcheck', label: '맞춤법 검사' },
      { value: 'unlimited', label: '무제한 글자 수' },
      { value: 'stats', label: '작성 기록과 일일 목표' },
      { value: 'organize', label: '폴더와 노트로 정리' },
      { value: 'sharing', label: '공유와 공개 범위 설정' },
      { value: 'theme', label: '테마와 디자인' },
      { value: 'other', label: '직접 입력', input: 'required', prompt: '어떤 점인지 알려주세요', pinned: true },
    ],
  },
  {
    id: 'dependence',
    kind: 'single',
    title: '타이피가 사라진다면 어떨 것 같나요?',
    hint: '하나만 골라 주세요',
    columns: 1,
    shuffle: false,
    options: [
      { value: 'very', label: '매우 아쉬울 것 같아요' },
      { value: 'somewhat', label: '조금 아쉬울 것 같아요' },
      { value: 'not', label: '별로 아쉽지 않을 것 같아요' },
      { value: 'inactive', label: '요즘은 잘 쓰지 않아요' },
    ],
  },
  {
    id: 'feedback',
    kind: 'text',
    title: '아쉬운 점이나 바라는 점이 있다면 알려주세요',
    hint: '선택 사항이에요',
    placeholder: '기능 요청, 불편한 점, 응원 무엇이든 좋아요',
  },
];

export type UserSurveyAnswer = {
  selected: string[];
  inputs: Record<string, string>;
  text: string;
};

export type UserSurveyDraft = Record<UserSurveyQuestionId, UserSurveyAnswer>;

export type UserSurveyValue = {
  genres: string[];
  genres_other: string;
  source: string;
  source_other: string;
  source_community: string;
  source_search: string;
  previous_tool: string;
  previous_tool_other: string;
  reason: string;
  reason_other: string;
  dependence: string;
  feedback: string;
};

export function createUserSurveyDraft(): UserSurveyDraft {
  const empty = (): UserSurveyAnswer => ({ selected: [], inputs: {}, text: '' });

  return {
    genres: empty(),
    source: empty(),
    previous_tool: empty(),
    reason: empty(),
    dependence: empty(),
    feedback: empty(),
  };
}

export function orderUserSurveyOptions(question: UserSurveyQuestion, random: () => number = Math.random): UserSurveyOption[] {
  if (question.kind === 'text') {
    return [];
  }

  const unpinned = question.options.filter((option) => !option.pinned);
  const pinned = question.options.filter((option) => option.pinned);
  const head = question.shuffle ? unpinned.toSorted(() => random() - 0.5) : unpinned;

  return [...head, ...pinned];
}

export function orderUserSurvey(random: () => number = Math.random): Record<UserSurveyQuestionId, UserSurveyOption[]> {
  return Object.fromEntries(USER_SURVEY_QUESTIONS.map((question) => [question.id, orderUserSurveyOptions(question, random)])) as Record<
    UserSurveyQuestionId,
    UserSurveyOption[]
  >;
}

export function selectUserSurveyOption(question: UserSurveyQuestion, answer: UserSurveyAnswer, value: string): string[] {
  if (question.kind === 'single') {
    return [value];
  }

  if (question.kind === 'multi') {
    if (answer.selected.includes(value)) {
      return answer.selected.filter((selected) => selected !== value);
    }

    if (question.max !== undefined && answer.selected.length >= question.max) {
      return answer.selected;
    }

    return [...answer.selected, value];
  }

  return answer.selected;
}

export function visibleUserSurveyInputs(question: UserSurveyQuestion, answer: UserSurveyAnswer): UserSurveyOption[] {
  if (question.kind === 'text') {
    return [];
  }

  return question.options.filter((option) => option.input && answer.selected.includes(option.value));
}

export function canAdvanceUserSurvey(question: UserSurveyQuestion, answer: UserSurveyAnswer): boolean {
  if (question.kind === 'text') {
    return true;
  }

  if (answer.selected.length === 0) {
    return false;
  }

  if (question.kind === 'multi' && question.max !== undefined && answer.selected.length > question.max) {
    return false;
  }

  return visibleUserSurveyInputs(question, answer).every(
    (option) => option.input !== 'required' || (answer.inputs[option.value] ?? '').trim() !== '',
  );
}

const inputOf = (answer: UserSurveyAnswer, value: string): string => {
  return answer.selected.includes(value) ? (answer.inputs[value] ?? '').trim() : '';
};

export function buildUserSurveyValue(draft: UserSurveyDraft): UserSurveyValue {
  return {
    genres: draft.genres.selected,
    genres_other: inputOf(draft.genres, 'other'),
    source: draft.source.selected[0] ?? '',
    source_other: inputOf(draft.source, 'other'),
    source_community: inputOf(draft.source, 'community'),
    source_search: inputOf(draft.source, 'search'),
    previous_tool: draft.previous_tool.selected[0] ?? '',
    previous_tool_other: inputOf(draft.previous_tool, 'other'),
    reason: draft.reason.selected[0] ?? '',
    reason_other: inputOf(draft.reason, 'other'),
    dependence: draft.dependence.selected[0] ?? '',
    feedback: draft.feedback.text.trim(),
  };
}

export function userSurveySnoozeUntil(now: Date): Date {
  return new Date(now.getTime() + USER_SURVEY_SNOOZE_DAYS * 24 * 60 * 60 * 1000);
}

export function isUserSurveySnoozed(until: string | null, now: Date): boolean {
  if (!until) {
    return false;
  }

  return new Date(until).getTime() >= now.getTime();
}
