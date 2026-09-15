import '@typie/lib/dayjs';

import dayjs from 'dayjs';
import type { PublicationState } from '@typie/lib/enums';
import type { Dayjs } from 'dayjs';

export type ScheduleParts = { date: Date; time: string };
export type PublishMode = 'now' | 'schedule';
export type PublishAction = 'publish' | 'schedule' | 'republish' | 'reschedule' | 'publishNow' | 'unpublish' | 'cancel';

export type PublicationSummary = {
  state: PublicationState;
  scheduledAt: string | null;
  publishedAt: string | null;
  hasUnpublishedChanges: boolean;
  tags: readonly string[];
  excerpt: string | null;
  thumbnailId: string | null;
} | null;

export type PublishDraft = {
  tags: readonly string[];
  excerpt: string;
  thumbnailId: string | null;
  mode: PublishMode;
  schedule: ScheduleParts;
};

export type ModifiedPublishFields = {
  tags: boolean;
  excerpt: boolean;
  thumbnail: boolean;
  schedule: boolean;
};

export type PublishActionButton = { label: string; action: PublishAction };

export type PublishView = {
  kind: 'draft' | 'published' | 'scheduled';
  statusLabel: string | null;
  modified: boolean;
  scheduleEditable: boolean;
  canGoBack: boolean;
  primary: (PublishActionButton & { enabled: boolean }) | null;
  secondary: PublishActionButton | null;
  destructive: PublishActionButton | null;
};

const TAG_SEPARATOR = /[,\n\r]/;

export const normalizeTagInput = (raw: string): string => raw.normalize('NFC').trim().replace(/^#+/, '').trim();

export const parseTagInput = (raw: string, existing: readonly string[]): string[] => {
  const next = [...existing];
  const seen = new Set(existing);
  for (const piece of raw.split(TAG_SEPARATOR)) {
    const tag = normalizeTagInput(piece);
    if (tag.length === 0 || seen.has(tag)) continue;
    seen.add(tag);
    next.push(tag);
  }
  return next;
};

export const duplicatedTagInput = (raw: string, existing: readonly string[]): string | null => {
  for (const piece of raw.split(TAG_SEPARATOR)) {
    const tag = normalizeTagInput(piece);
    if (tag.length > 0 && existing.includes(tag)) return tag;
  }
  return null;
};

const DATE_KEY = 'YYYY-MM-DD';

const partsOf = (at: Dayjs): ScheduleParts => ({ date: dayjs(at.format(DATE_KEY)).toDate(), time: at.format('HH:mm') });

export const defaultScheduleParts = (now: Dayjs): ScheduleParts => partsOf(now.kst().add(1, 'hour').startOf('hour'));

export const scheduleParts = (iso: string): ScheduleParts => partsOf(dayjs(iso).kst());

export const composeScheduledAt = (parts: ScheduleParts): string =>
  dayjs.kst(`${dayjs(parts.date).format(DATE_KEY)}T${parts.time}`).toISOString();

export const isScheduleInFuture = (parts: ScheduleParts, now: Dayjs): boolean => dayjs(composeScheduledAt(parts)).isAfter(now);

export const scheduleDayPassed = (date: Date, now: Dayjs): boolean => dayjs(date).isBefore(now.kst().startOf('day'), 'day');

export const scheduleHourPassed = (date: Date, hour: number, now: Dayjs): boolean =>
  !isScheduleInFuture({ date, time: `${String(hour).padStart(2, '0')}:59` }, now);

export const scheduleMinutePassed = (date: Date, hour: number, minute: number, now: Dayjs): boolean =>
  !isScheduleInFuture({ date, time: `${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}` }, now);

export const nextSchedulableParts = (parts: ScheduleParts, now: Dayjs): ScheduleParts => {
  if (isScheduleInFuture(parts, now)) return parts;

  const at = now.kst().add(1, 'minute').startOf('minute');
  return { date: dayjs(at.format(DATE_KEY)).toDate(), time: at.format('HH:mm') };
};

export const formatPublishTime = (iso: string, now: Dayjs): string => {
  const at = dayjs(iso).kst();
  const days = at.startOf('day').diff(now.kst().startOf('day'), 'day');
  const time = at.format('A h:mm');

  if (days === 0) return `오늘 ${time}`;
  if (days === 1) return `내일 ${time}`;
  return `${at.format('M월 D일 (ddd)')} ${time}`;
};

const sameTags = (a: readonly string[], b: readonly string[]): boolean =>
  a.length === b.length && a.every((tag, index) => tag === b[index]);

const NO_MODIFIED_FIELDS: ModifiedPublishFields = {
  tags: false,
  excerpt: false,
  thumbnail: false,
  schedule: false,
};

export const modifiedPublishFields = (publication: PublicationSummary, draft: PublishDraft): ModifiedPublishFields => {
  if (!publication || publication.state === 'UNPUBLISHED') return NO_MODIFIED_FIELDS;

  const scheduled = publication.state === 'SCHEDULED';

  return {
    tags: !sameTags(publication.tags, draft.tags),
    excerpt: (publication.excerpt ?? '') !== draft.excerpt.trim(),
    thumbnail: publication.thumbnailId !== draft.thumbnailId,
    schedule:
      scheduled &&
      (draft.mode === 'now' ||
        publication.scheduledAt === null ||
        composeScheduledAt(draft.schedule) !== dayjs(publication.scheduledAt).toISOString()),
  };
};

export const isPublishDraftDirty = (publication: NonNullable<PublicationSummary>, draft: PublishDraft): boolean => {
  if (publication.state === 'UNPUBLISHED') return false;

  return publication.hasUnpublishedChanges || Object.values(modifiedPublishFields(publication, draft)).includes(true);
};

export const resolvePublishView = (publication: PublicationSummary, draft: PublishDraft): PublishView => {
  if (!publication || publication.state === 'UNPUBLISHED') {
    return {
      kind: 'draft',
      statusLabel: null,
      modified: false,
      scheduleEditable: true,
      canGoBack: true,
      primary:
        draft.mode === 'schedule'
          ? { label: '예약 발행', action: 'schedule', enabled: true }
          : { label: '발행', action: 'publish', enabled: true },
      secondary: null,
      destructive: null,
    };
  }

  const dirty = isPublishDraftDirty(publication, draft);

  if (publication.state === 'SCHEDULED') {
    const immediate = draft.mode === 'now';

    return {
      kind: 'scheduled',
      statusLabel: '예약됨',
      modified: publication.hasUnpublishedChanges,
      scheduleEditable: true,
      canGoBack: false,
      primary: immediate
        ? { label: '지금 발행', action: 'publishNow', enabled: true }
        : { label: '예약에 반영', action: 'reschedule', enabled: dirty },
      secondary: immediate ? null : { label: '지금 발행', action: 'publishNow' },
      destructive: { label: '예약 취소', action: 'cancel' },
    };
  }

  return {
    kind: 'published',
    statusLabel: '발행됨',
    modified: publication.hasUnpublishedChanges,
    scheduleEditable: false,
    canGoBack: false,
    primary: { label: '다시 발행', action: 'republish', enabled: dirty },
    secondary: null,
    destructive: { label: '발행 취소', action: 'unpublish' },
  };
};

export type BulkScheduleMode = PublishMode | 'keep';

export type BulkDocumentSummary = { id: string; publication: PublicationSummary };

export type BulkScheduleInitial = { mode: BulkScheduleMode; schedule: ScheduleParts };

export type BulkPublishDraft = {
  addTags: readonly string[];
  removeTags: readonly string[];
  scheduleMode: BulkScheduleMode;
  schedule: ScheduleParts;
};

export type BulkPublishView = {
  counts: { unpublished: number; scheduled: number; published: number };
  mixed: boolean;
  scheduleEditable: boolean;
  canGoBack: boolean;
  modified: ModifiedPublishFields;
  targetIds: string[];
  primary: { label: string; enabled: boolean };
  destructive: { label: string; action: 'unpublish' | 'cancel'; documentIds: string[] }[];
  publishNowConfirm: boolean;
};

export const sharedValue: <T>(values: readonly T[]) => T | undefined = (values) =>
  values.length > 0 && values.every((value) => value === values[0]) ? values[0] : undefined;

export const partitionTags = (lists: readonly (readonly string[])[]): { common: string[]; partial: { tag: string; count: number }[] } => {
  const counts = new Map<string, number>();
  for (const list of lists) {
    for (const tag of new Set(list)) {
      counts.set(tag, (counts.get(tag) ?? 0) + 1);
    }
  }

  const common: string[] = [];
  const partial: { tag: string; count: number }[] = [];
  for (const [tag, count] of counts) {
    if (count === lists.length) {
      common.push(tag);
    } else {
      partial.push({ tag, count });
    }
  }

  return { common, partial };
};

export const bulkTagDraft = (
  base: { common: readonly string[]; partial: readonly string[] },
  view: { tags: readonly string[]; partial: readonly string[] },
): { addTags: string[]; removeTags: string[] } => ({
  addTags: view.tags.filter((tag) => !base.common.includes(tag)),
  removeTags: [
    ...base.common.filter((tag) => !view.tags.includes(tag)),
    ...base.partial.filter((tag) => !view.partial.includes(tag) && !view.tags.includes(tag)),
  ],
});

export const initialBulkSchedule = (documents: readonly BulkDocumentSummary[], now: Dayjs): BulkScheduleInitial => {
  const scheduledAts = documents
    .map(({ publication }) => (publication?.state === 'SCHEDULED' ? publication.scheduledAt : undefined))
    .filter((at): at is string => typeof at === 'string');

  if (scheduledAts.length === 0) return { mode: 'now', schedule: defaultScheduleParts(now) };

  const at = sharedValue(scheduledAts);
  if (at && scheduledAts.length === documents.length) return { mode: 'schedule', schedule: scheduleParts(at) };

  return { mode: 'keep', schedule: defaultScheduleParts(now) };
};

export const bulkModifiedFields = (draft: BulkPublishDraft, initial: BulkScheduleInitial): ModifiedPublishFields => ({
  tags: draft.addTags.length > 0 || draft.removeTags.length > 0,
  excerpt: false,
  thumbnail: false,
  schedule:
    draft.scheduleMode !== initial.mode ||
    (draft.scheduleMode === 'schedule' && composeScheduledAt(draft.schedule) !== composeScheduledAt(initial.schedule)),
});

export const bulkTargetDocumentIds = (documents: readonly BulkDocumentSummary[], modified: ModifiedPublishFields): string[] => {
  const touchesPublished = modified.tags;
  const touchesScheduled = touchesPublished || modified.schedule;

  return documents
    .filter(({ publication }) => {
      if (!publication || publication.state === 'UNPUBLISHED') return true;
      if (publication.state === 'SCHEDULED') return touchesScheduled || publication.hasUnpublishedChanges;
      return touchesPublished || publication.hasUnpublishedChanges;
    })
    .map(({ id }) => id);
};

export const resolveBulkPublishView = (
  documents: readonly BulkDocumentSummary[],
  draft: BulkPublishDraft,
  initial: BulkScheduleInitial,
): BulkPublishView => {
  const counts = { unpublished: 0, scheduled: 0, published: 0 };
  for (const { publication } of documents) {
    if (!publication || publication.state === 'UNPUBLISHED') counts.unpublished += 1;
    else if (publication.state === 'SCHEDULED') counts.scheduled += 1;
    else counts.published += 1;
  }

  const modified = bulkModifiedFields(draft, initial);
  const targetIds = bulkTargetDocumentIds(documents, modified);
  const count = targetIds.length;
  const mixed = counts.scheduled + counts.published > 0;
  const editable = counts.unpublished + counts.scheduled > 0;

  const destructive: BulkPublishView['destructive'] = [];
  if (counts.published > 0) {
    destructive.push({
      label: '발행 취소',
      action: 'unpublish',
      documentIds: documents.filter(({ publication }) => publication?.state === 'PUBLISHED').map(({ id }) => id),
    });
  }
  if (counts.scheduled > 0) {
    destructive.push({
      label: '예약 취소',
      action: 'cancel',
      documentIds: documents.filter(({ publication }) => publication?.state === 'SCHEDULED').map(({ id }) => id),
    });
  }

  return {
    counts,
    mixed,
    scheduleEditable: editable,
    canGoBack: !mixed,
    modified,
    targetIds,
    primary: {
      label: mixed ? `글 ${count}개에 반영` : draft.scheduleMode === 'schedule' ? `글 ${count}개 예약 발행` : `글 ${count}개 발행`,
      enabled: count > 0,
    },
    destructive,
    publishNowConfirm: draft.scheduleMode === 'now' && counts.scheduled > 0,
  };
};

export const bulkScheduledAtInput = (draft: BulkPublishDraft, modified: ModifiedPublishFields): string | null | undefined => {
  if (!modified.schedule) return undefined;
  if (draft.scheduleMode === 'now') return null;
  if (draft.scheduleMode === 'schedule') return composeScheduledAt(draft.schedule);
  return undefined;
};

export const bulkPublishSuccessMessage = (view: BulkPublishView, mode: BulkScheduleMode): string => {
  const count = view.targetIds.length;
  if (view.mixed) return `글 ${count}개에 반영됐어요`;
  return mode === 'schedule' ? `글 ${count}개의 발행이 예약됐어요` : `글 ${count}개가 발행됐어요`;
};

const ERROR_MESSAGES: Record<string, string> = {
  publication_empty_document: '빈 문서는 발행할 수 없어요.',
  publication_projection_degraded: '지금은 발행할 수 없는 문서예요. 잠시 후 다시 시도해 주세요.',
  publication_scheduled_in_past: '예약 시각은 지금 이후여야 해요.',
  publication_already_published: '이미 발행된 글이에요.',
  publication_not_published: '발행 중인 글이 아니에요.',
  publication_not_scheduled: '예약된 글이 아니에요.',
  publication_unpublish_required: '공개 방식을 바꾸려면 먼저 발행을 취소해야 해요.',
  publication_link_share_blocked: '공개 방식을 바꾸려면 먼저 발행을 취소해야 해요.',
  publication_no_documents: '발행할 글이 없어요.',
  site_slug_already_exists: '이미 존재하는 스페이스 주소예요.',
  site_pin_limit: '고정은 3개까지 할 수 있어요.',
  site_link_invalid: '링크 주소는 http 또는 https로 시작해야 해요.',
};

export const publicationErrorMessage = (code: string | null): string =>
  (code === null ? undefined : ERROR_MESSAGES[code]) ?? '잠시 후 다시 시도해 주세요';
