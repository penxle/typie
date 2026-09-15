import dayjs from 'dayjs';
import { describe, expect, it } from 'vitest';
import {
  bulkModifiedFields,
  bulkPublishSuccessMessage,
  bulkScheduledAtInput,
  bulkTagDraft,
  bulkTargetDocumentIds,
  composeScheduledAt,
  defaultScheduleParts,
  formatPublishTime,
  initialBulkSchedule,
  isPublishDraftDirty,
  isScheduleInFuture,
  modifiedPublishFields,
  parseTagInput,
  partitionTags,
  publicationErrorMessage,
  resolveBulkPublishView,
  resolvePublishView,
  scheduleParts,
  sharedValue,
} from './publish-form.ts';
import type { BulkDocumentSummary, BulkPublishDraft, PublicationSummary, PublishDraft } from './publish-form.ts';

describe('parseTagInput', () => {
  it('쉼표로 나누고 공백·빈 조각·중복을 버린다', () => {
    expect(parseTagInput(' 에세이, 일기 ,, 에세이', ['소설'])).toEqual(['소설', '에세이', '일기']);
  });

  it('NFC로 정규화해 같은 글자를 한 태그로 본다', () => {
    expect(parseTagInput('\u{1112}\u{1161}\u{11AB}', [])).toEqual(['한']);
    expect(parseTagInput('\u{1112}\u{1161}\u{11AB}', ['한'])).toEqual(['한']);
  });

  it('입력이 비면 기존 태그를 그대로 돌려준다', () => {
    expect(parseTagInput('', ['a', 'b'])).toEqual(['a', 'b']);
  });

  it('앞에 붙은 #을 떼고 줄바꿈으로도 나눈다', () => {
    expect(parseTagInput('#가을\n##편지, 산책', [])).toEqual(['가을', '편지', '산책']);
  });
});

describe('schedule parts', () => {
  it('KST 벽시계를 UTC ISO로 합성한다', () => {
    expect(composeScheduledAt({ date: new Date(2026, 8, 8), time: '14:00' })).toBe('2026-09-08T05:00:00.000Z');
  });

  it('ISO를 KST 달력일과 시각으로 되돌린다', () => {
    const parts = scheduleParts('2026-09-08T05:00:00.000Z');
    expect(dayjs(parts.date).format('YYYY-MM-DD')).toBe('2026-09-08');
    expect(parts.time).toBe('14:00');
  });

  it('기본값은 KST 다음 정시이며 자정을 넘기면 날짜가 바뀐다', () => {
    const parts = defaultScheduleParts(dayjs('2026-09-08T23:30:00+09:00'));
    expect(dayjs(parts.date).format('YYYY-MM-DD')).toBe('2026-09-09');
    expect(parts.time).toBe('00:00');
  });

  it('지금 이전 시각을 가려낸다', () => {
    const now = dayjs('2026-09-08T05:30:00.000Z');
    expect(isScheduleInFuture({ date: new Date(2026, 8, 8), time: '14:00' }, now)).toBe(false);
    expect(isScheduleInFuture({ date: new Date(2026, 8, 8), time: '15:00' }, now)).toBe(true);
  });
});

const draft = (patch: Partial<PublishDraft> = {}): PublishDraft => ({
  tags: [],
  excerpt: '',
  thumbnailId: null,
  mode: 'now',
  schedule: { date: new Date(2026, 8, 8), time: '14:00' },
  ...patch,
});

const published = (patch: Partial<NonNullable<PublicationSummary>> = {}): NonNullable<PublicationSummary> => ({
  state: 'PUBLISHED',
  scheduledAt: null,
  publishedAt: '2026-09-08T05:00:00.000Z',
  hasUnpublishedChanges: false,
  tags: [],
  excerpt: null,
  thumbnailId: null,
  ...patch,
});

describe('formatPublishTime', () => {
  it('오늘과 내일은 말로, 그 밖은 날짜로 적는다', () => {
    const now = dayjs('2026-09-08T05:00:00.000Z');
    expect(formatPublishTime('2026-09-08T09:30:00.000Z', now)).toBe('오늘 오후 6:30');
    expect(formatPublishTime('2026-09-09T00:00:00.000Z', now)).toBe('내일 오전 9:00');
    expect(formatPublishTime('2026-09-11T12:00:00.000Z', now)).toBe('9월 11일 (금) 오후 9:00');
  });
});

describe('isPublishDraftDirty', () => {
  it('발행된 글은 메타가 바뀌거나 본문이 수정되면 바뀐 것이다', () => {
    expect(isPublishDraftDirty(published(), draft())).toBe(false);
    expect(isPublishDraftDirty(published(), draft({ tags: ['가을'] }))).toBe(true);
    expect(isPublishDraftDirty(published({ hasUnpublishedChanges: true }), draft())).toBe(true);
    expect(isPublishDraftDirty(published({ excerpt: '여름' }), draft({ excerpt: ' 여름 ' }))).toBe(false);
  });

  it('예약된 글은 시각·지금 발행 전환도 바뀐 것으로 본다', () => {
    const scheduled = published({ state: 'SCHEDULED', publishedAt: null, scheduledAt: '2026-09-08T05:00:00.000Z' });
    expect(isPublishDraftDirty(scheduled, draft({ mode: 'schedule' }))).toBe(false);
    expect(isPublishDraftDirty(scheduled, draft({ mode: 'now' }))).toBe(true);
    expect(isPublishDraftDirty(scheduled, draft({ mode: 'schedule', schedule: { date: new Date(2026, 8, 8), time: '15:00' } }))).toBe(true);
  });
});

describe('modifiedPublishFields', () => {
  it('발행된 글은 바뀐 메타 항목만 집어낸다', () => {
    expect(modifiedPublishFields(published(), draft())).toEqual({
      tags: false,
      excerpt: false,
      thumbnail: false,
      schedule: false,
    });

    const fields = modifiedPublishFields(published({ tags: ['여름'] }), draft({ tags: ['여름'], thumbnailId: 'image-1' }));
    expect(fields.thumbnail).toBe(true);
    expect(fields.tags).toBe(false);
  });

  it('발행 전에는 아무것도 수정으로 보지 않는다', () => {
    expect(modifiedPublishFields(null, draft({ tags: ['가을'] })).tags).toBe(false);
    expect(modifiedPublishFields(published({ state: 'UNPUBLISHED' }), draft({ tags: ['가을'] })).tags).toBe(false);
  });

  it('예약된 글만 시각을 수정으로 본다', () => {
    const scheduled = published({ state: 'SCHEDULED', publishedAt: null, scheduledAt: '2026-09-08T05:00:00.000Z' });

    expect(modifiedPublishFields(scheduled, draft({ mode: 'schedule' })).schedule).toBe(false);
    expect(modifiedPublishFields(scheduled, draft({ mode: 'now' })).schedule).toBe(true);
    expect(modifiedPublishFields(published(), draft({ mode: 'now' })).schedule).toBe(false);
  });
});

describe('resolvePublishView', () => {
  it('발행 전에는 뒤로 갈 수 있고 모드에 따라 발행/예약 발행이다', () => {
    expect(resolvePublishView(null, draft())).toEqual({
      kind: 'draft',
      statusLabel: null,
      modified: false,
      scheduleEditable: true,
      canGoBack: true,
      primary: { label: '발행', action: 'publish', enabled: true },
      secondary: null,
      destructive: null,
    });
    expect(resolvePublishView(null, draft({ mode: 'schedule' })).primary).toEqual({
      label: '예약 발행',
      action: 'schedule',
      enabled: true,
    });
  });

  it('발행 취소된 글도 발행 전과 같다', () => {
    const view = resolvePublishView(published({ state: 'UNPUBLISHED' }), draft());
    expect(view.kind).toBe('draft');
    expect(view.primary).toEqual({ label: '발행', action: 'publish', enabled: true });
  });

  it('발행된 글은 시각을 잠그고 바뀐 게 있어야 다시 발행할 수 있다', () => {
    const view = resolvePublishView(published(), draft());
    expect(view.kind).toBe('published');
    expect(view.statusLabel).toBe('발행됨');
    expect(view.scheduleEditable).toBe(false);
    expect(view.canGoBack).toBe(false);
    expect(view.primary).toEqual({ label: '다시 발행', action: 'republish', enabled: false });
    expect(view.destructive).toEqual({ label: '발행 취소', action: 'unpublish' });
    expect(resolvePublishView(published(), draft({ tags: ['가을'] })).primary?.enabled).toBe(true);
  });

  it('예약된 글은 고칠 수 있고 지금 발행과 예약에 반영을 함께 준다', () => {
    const scheduled = published({
      state: 'SCHEDULED',
      publishedAt: null,
      scheduledAt: '2026-09-08T05:00:00.000Z',
      hasUnpublishedChanges: true,
    });
    const view = resolvePublishView(scheduled, draft({ mode: 'schedule' }));
    expect(view.kind).toBe('scheduled');
    expect(view.modified).toBe(true);
    expect(view.scheduleEditable).toBe(true);
    expect(view.canGoBack).toBe(false);
    expect(view.primary).toEqual({ label: '예약에 반영', action: 'reschedule', enabled: true });
    expect(view.secondary).toEqual({ label: '지금 발행', action: 'publishNow' });
    expect(view.destructive).toEqual({ label: '예약 취소', action: 'cancel' });

    const immediate = resolvePublishView(scheduled, draft({ mode: 'now' }));
    expect(immediate.primary).toEqual({ label: '지금 발행', action: 'publishNow', enabled: true });
    expect(immediate.secondary).toBeNull();
  });
});

describe('publicationErrorMessage', () => {
  it('아는 코드는 전용 문구, 모르는 코드와 null은 기본 문구다', () => {
    expect(publicationErrorMessage('site_pin_limit')).toBe('고정은 3개까지 할 수 있어요.');
    expect(publicationErrorMessage('publication_link_share_blocked')).toBe('공개 방식을 바꾸려면 먼저 발행을 취소해야 해요.');
    expect(publicationErrorMessage('unknown_code')).toBe('잠시 후 다시 시도해 주세요');
    expect(publicationErrorMessage(null)).toBe('잠시 후 다시 시도해 주세요');
  });
});

describe('일괄 발행', () => {
  const published = (id: string, over: Partial<NonNullable<PublicationSummary>> = {}): BulkDocumentSummary => ({
    id,
    publication: {
      state: 'PUBLISHED',
      scheduledAt: null,
      publishedAt: '2026-09-10T03:00:00.000Z',
      hasUnpublishedChanges: false,
      tags: [],
      excerpt: null,
      thumbnailId: null,
      ...over,
    },
  });

  const scheduled = (id: string, at: string, over: Partial<NonNullable<PublicationSummary>> = {}): BulkDocumentSummary =>
    published(id, { state: 'SCHEDULED', scheduledAt: at, publishedAt: null, ...over });

  const unpublished = (id: string): BulkDocumentSummary => ({ id, publication: null });

  const now = dayjs('2026-09-15T12:00:00+09:00');

  const untouched = (initial: ReturnType<typeof initialBulkSchedule>): BulkPublishDraft => ({
    addTags: [],
    removeTags: [],
    scheduleMode: initial.mode,
    schedule: initial.schedule,
  });

  describe('sharedValue', () => {
    it('전부 같으면 그 값을, 하나라도 다르거나 비어 있으면 undefined를 돌려준다', () => {
      expect(sharedValue(['a', 'a'])).toBe('a');
      expect(sharedValue(['a', 'b'])).toBeUndefined();
      expect(sharedValue([])).toBeUndefined();
      expect(sharedValue([null, null])).toBeNull();
    });
  });

  describe('partitionTags', () => {
    it('전부가 가진 태그와 일부만 가진 태그를 첫 등장 순서로 나눈다', () => {
      expect(partitionTags([['에세이', '일기'], ['일기', '소설'], ['일기']])).toEqual({
        common: ['일기'],
        partial: [
          { tag: '에세이', count: 1 },
          { tag: '소설', count: 1 },
        ],
      });
    });

    it('목록이 하나면 전부 공통이다', () => {
      expect(partitionTags([['a']])).toEqual({ common: ['a'], partial: [] });
    });
  });

  describe('bulkTagDraft', () => {
    const base = { common: ['일기'], partial: ['에세이'] };

    it('새로 넣은 태그는 add, 공통에서 뺀 태그와 일부에서 뺀 태그는 remove다', () => {
      expect(bulkTagDraft(base, { tags: ['소설'], partial: [] })).toEqual({ addTags: ['소설'], removeTags: ['일기', '에세이'] });
    });

    it('일부만 가진 태그를 다시 추가하면 add로만 기록한다', () => {
      expect(bulkTagDraft(base, { tags: ['일기', '에세이'], partial: [] })).toEqual({ addTags: ['에세이'], removeTags: [] });
    });

    it('건드리지 않으면 비어 있다', () => {
      expect(bulkTagDraft(base, { tags: ['일기'], partial: ['에세이'] })).toEqual({ addTags: [], removeTags: [] });
    });
  });

  describe('initialBulkSchedule', () => {
    it('예약된 글이 없으면 지금이다', () => {
      expect(initialBulkSchedule([unpublished('A'), published('B')], now).mode).toBe('now');
    });

    it('전부 같은 시각에 예약돼 있으면 그 시각이다', () => {
      const at = '2026-09-20T01:00:00.000Z';
      const initial = initialBulkSchedule([scheduled('A', at), scheduled('B', at)], now);
      expect(initial.mode).toBe('schedule');
      expect(composeScheduledAt(initial.schedule)).toBe(at);
    });

    it('예약 시각이 다르거나 미발행이 섞이면 유지다', () => {
      expect(initialBulkSchedule([scheduled('A', '2026-09-20T01:00:00.000Z'), scheduled('B', '2026-09-21T01:00:00.000Z')], now).mode).toBe(
        'keep',
      );
      expect(initialBulkSchedule([scheduled('A', '2026-09-20T01:00:00.000Z'), unpublished('B')], now).mode).toBe('keep');
    });
  });

  describe('bulkTargetDocumentIds', () => {
    const docs = [
      unpublished('U'),
      scheduled('S', '2026-09-20T01:00:00.000Z'),
      published('P'),
      published('M', { hasUnpublishedChanges: true }),
    ];

    it('아무것도 안 고르면 미발행 글과 수정된 글만 대상이다', () => {
      const initial = initialBulkSchedule(docs, now);
      expect(bulkTargetDocumentIds(docs, bulkModifiedFields(untouched(initial), initial))).toEqual(['U', 'M']);
    });

    it('시각만 바꾸면 발행된 글은 빠지고 예약된 글은 들어간다', () => {
      const initial = initialBulkSchedule(docs, now);
      const modified = bulkModifiedFields({ ...untouched(initial), scheduleMode: 'now' }, initial);
      expect(bulkTargetDocumentIds(docs, modified)).toEqual(['U', 'S', 'M']);
    });

    it('태그를 바꾸면 전부 대상이다', () => {
      const initial = initialBulkSchedule(docs, now);
      const modified = bulkModifiedFields({ ...untouched(initial), addTags: ['a'] }, initial);
      expect(bulkTargetDocumentIds(docs, modified)).toEqual(['U', 'S', 'P', 'M']);
    });
  });

  describe('resolveBulkPublishView', () => {
    it('전부 미발행이면 발행 라벨과 뒤로 가기가 있고 파괴 버튼이 없다', () => {
      const docs = [unpublished('A'), unpublished('B')];
      const initial = initialBulkSchedule(docs, now);
      const view = resolveBulkPublishView(docs, untouched(initial), initial);
      expect(view.primary).toEqual({ label: '글 2개 발행', enabled: true });
      expect(view.canGoBack).toBe(true);
      expect(view.destructive).toEqual([]);
      expect(resolveBulkPublishView(docs, { ...untouched(initial), scheduleMode: 'schedule' }, initial).primary.label).toBe(
        '글 2개 예약 발행',
      );
    });

    it('상태가 섞이면 반영 라벨, 대상 수, 파괴 버튼 둘, 뒤로 가기 없음이다', () => {
      const docs = [unpublished('U'), scheduled('S', '2026-09-20T01:00:00.000Z'), published('P')];
      const initial = initialBulkSchedule(docs, now);
      const view = resolveBulkPublishView(docs, untouched(initial), initial);
      expect(view.primary).toEqual({ label: '글 1개에 반영', enabled: true });
      expect(view.canGoBack).toBe(false);
      expect(view.destructive).toEqual([
        { label: '발행 취소', action: 'unpublish', documentIds: ['P'] },
        { label: '예약 취소', action: 'cancel', documentIds: ['S'] },
      ]);
      expect(view.publishNowConfirm).toBe(false);
      expect(resolveBulkPublishView(docs, { ...untouched(initial), scheduleMode: 'now' }, initial).publishNowConfirm).toBe(true);
    });

    it('바뀐 것이 없으면 주 버튼이 꺼진다', () => {
      const docs = [published('P')];
      const initial = initialBulkSchedule(docs, now);
      expect(resolveBulkPublishView(docs, untouched(initial), initial).primary).toEqual({ label: '글 0개에 반영', enabled: false });
    });
  });

  describe('bulkScheduledAtInput', () => {
    it('안 바꿨으면 생략, 지금이면 null, 예약이면 ISO다', () => {
      const initial = { mode: 'keep' as const, schedule: defaultScheduleParts(now) };
      const draft = untouched(initial);
      expect(bulkScheduledAtInput(draft, bulkModifiedFields(draft, initial))).toBeUndefined();
      const toNow = { ...draft, scheduleMode: 'now' as const };
      expect(bulkScheduledAtInput(toNow, bulkModifiedFields(toNow, initial))).toBeNull();
      const toSchedule = { ...draft, scheduleMode: 'schedule' as const, schedule: { date: new Date(2026, 8, 20), time: '10:00' } };
      expect(bulkScheduledAtInput(toSchedule, bulkModifiedFields(toSchedule, initial))).toBe('2026-09-20T01:00:00.000Z');
    });
  });

  describe('bulkPublishSuccessMessage', () => {
    it('전부 미발행이면 발행·예약 문구, 섞이면 반영 문구다', () => {
      const docs = [unpublished('A'), unpublished('B')];
      const initial = initialBulkSchedule(docs, now);
      const view = resolveBulkPublishView(docs, untouched(initial), initial);
      expect(bulkPublishSuccessMessage(view, 'now')).toBe('글 2개가 발행됐어요');
      expect(bulkPublishSuccessMessage(view, 'schedule')).toBe('글 2개의 발행이 예약됐어요');
      const mixed = [unpublished('A'), published('B')];
      const mixedInitial = initialBulkSchedule(mixed, now);
      expect(bulkPublishSuccessMessage(resolveBulkPublishView(mixed, untouched(mixedInitial), mixedInitial), 'now')).toBe(
        '글 1개에 반영됐어요',
      );
    });
  });
});
