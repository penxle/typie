import dayjs from 'dayjs';
import { describe, expect, it } from 'vitest';
import {
  composeScheduledAt,
  defaultScheduleParts,
  formatPublishTime,
  isPublishDraftDirty,
  isScheduleInFuture,
  modifiedPublishFields,
  parseTagInput,
  publicationErrorMessage,
  resolvePublishView,
  scheduleParts,
} from './publish-form.ts';
import type { PublicationSummary, PublishDraft } from './publish-form.ts';

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
  spaceId: 'space-1',
  collectionId: null,
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
  spaceId: 'space-1',
  collectionId: null,
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

  it('예약된 글은 스페이스·시각·지금 발행 전환도 바뀐 것으로 본다', () => {
    const scheduled = published({ state: 'SCHEDULED', publishedAt: null, scheduledAt: '2026-09-08T05:00:00.000Z' });
    expect(isPublishDraftDirty(scheduled, draft({ mode: 'schedule' }))).toBe(false);
    expect(isPublishDraftDirty(scheduled, draft({ mode: 'now' }))).toBe(true);
    expect(isPublishDraftDirty(scheduled, draft({ mode: 'schedule', spaceId: 'space-2' }))).toBe(true);
    expect(isPublishDraftDirty(scheduled, draft({ mode: 'schedule', schedule: { date: new Date(2026, 8, 8), time: '15:00' } }))).toBe(true);
  });
});

describe('modifiedPublishFields', () => {
  it('발행된 글은 바뀐 메타 항목만 집어낸다', () => {
    expect(modifiedPublishFields(published(), draft())).toEqual({
      space: false,
      collection: false,
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

  it('예약된 글만 스페이스·시각을 수정으로 본다', () => {
    const scheduled = published({ state: 'SCHEDULED', publishedAt: null, scheduledAt: '2026-09-08T05:00:00.000Z' });

    expect(modifiedPublishFields(scheduled, draft({ mode: 'schedule' })).schedule).toBe(false);
    expect(modifiedPublishFields(scheduled, draft({ mode: 'now' })).schedule).toBe(true);
    expect(modifiedPublishFields(scheduled, draft({ mode: 'schedule', spaceId: 'space-2' })).space).toBe(true);
    expect(modifiedPublishFields(published(), draft({ spaceId: 'space-2' })).space).toBe(false);
  });
});

describe('resolvePublishView', () => {
  it('발행 전에는 뒤로 갈 수 있고 모드에 따라 발행/예약 발행이다', () => {
    expect(resolvePublishView(null, draft())).toEqual({
      kind: 'draft',
      statusLabel: null,
      modified: false,
      spaceSelectable: true,
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

  it('발행된 글은 스페이스를 잠그고 바뀐 게 있어야 다시 발행할 수 있다', () => {
    const view = resolvePublishView(published(), draft());
    expect(view.kind).toBe('published');
    expect(view.statusLabel).toBe('발행됨');
    expect(view.spaceSelectable).toBe(false);
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
    expect(view.spaceSelectable).toBe(true);
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
    expect(publicationErrorMessage('space_pin_limit')).toBe('고정은 3개까지 할 수 있어요.');
    expect(publicationErrorMessage('publication_link_share_blocked')).toBe('공개 방식을 바꾸려면 먼저 발행을 취소해야 해요.');
    expect(publicationErrorMessage('unknown_code')).toBe('잠시 후 다시 시도해 주세요');
    expect(publicationErrorMessage(null)).toBe('잠시 후 다시 시도해 주세요');
  });
});
