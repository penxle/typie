import '@typie/lib/dayjs';

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ENTITY_ICON_COLORS, ENTITY_ICON_NAMES } from '@typie/lib/catalogs';
import dayjs from 'dayjs';
import {
  COMMENT_PAGE_SIZE,
  CreateDocumentsInput,
  CreateFoldersInput,
  CreateNotesInput,
  DeleteEntitiesInput,
  DeleteGoalsInput,
  DeleteNotesInput,
  DOCUMENT_WINDOW_DEFAULT,
  DOCUMENT_WINDOW_MAX,
  DuplicateDocumentsInput,
  entityRefKind,
  entityUrl,
  kstDate,
  kstDueDate,
  ListEntitiesInput,
  MoveEntitiesInput,
  NoteLinksInput,
  notePreview,
  pageOf,
  preorder,
  ReadCommentsInput,
  ReadDocumentInput,
  ReadNoteInput,
  ReadSharingInput,
  RecoverEntitiesInput,
  SearchEntitiesInput,
  SetGoalsInput,
  snippetOf,
  TRASH_PAGE_SIZE,
  UpdateDocumentsInput,
  UpdateFoldersInput,
  UpdateIconsInput,
  UpdateNotesInput,
  UpdateSharingInput,
  validIcon,
  windowOf,
  withinDays,
} from './prism-workspace-core.ts';

test('입력 미러: 유효·불능', () => {
  assert.deepEqual(SearchEntitiesInput.parse({ query: '해변' }), { query: '해변' });
  assert.equal(SearchEntitiesInput.safeParse({ query: '' }).success, false);
  assert.deepEqual(ListEntitiesInput.parse({}), { recursive: true, offset: 0, limit: 50 });
  assert.deepEqual(ListEntitiesInput.parse({ folderId: 'F1', recursive: false, offset: 50, limit: 100 }), {
    folderId: 'F1',
    recursive: false,
    offset: 50,
    limit: 100,
  });
  assert.equal(ListEntitiesInput.safeParse({ offset: -1 }).success, false);
  assert.equal(ListEntitiesInput.safeParse({ limit: 0 }).success, false);
  assert.equal(ListEntitiesInput.safeParse({ limit: 101 }).success, false);
  assert.equal(ReadDocumentInput.safeParse({}).success, false);
  assert.deepEqual(ReadDocumentInput.parse({ id: 'D1' }), { id: 'D1', offset: 0, length: DOCUMENT_WINDOW_DEFAULT });
  assert.deepEqual(ReadDocumentInput.parse({ id: 'D1', offset: 2000, length: 5000 }), {
    id: 'D1',
    offset: 2000,
    length: 5000,
  });
  assert.equal(ReadDocumentInput.safeParse({ id: 'D1', offset: -1 }).success, false);
  assert.equal(ReadDocumentInput.safeParse({ id: 'D1', length: 0 }).success, false);
  assert.equal(ReadDocumentInput.safeParse({ id: 'D1', length: DOCUMENT_WINDOW_MAX + 1 }).success, false);
  assert.equal(ReadDocumentInput.safeParse({ id: 'D1', offset: 1.5 }).success, false);
});

test('preorder: 페이지 자식을 뿌리로 전위 순회하고, 형제 순서는 온 순서(SQL order 정렬)를 지킨다', () => {
  const descendants = [
    { id: 'A1', parentId: 'A' },
    { id: 'B1', parentId: 'B' },
    { id: 'A2', parentId: 'A' },
    { id: 'A1a', parentId: 'A1' },
    { id: 'X', parentId: 'orphan' },
  ];
  assert.deepEqual(preorder(['A', 'B'], descendants), [
    { id: 'A', depth: 0 },
    { id: 'A1', depth: 1 },
    { id: 'A1a', depth: 2 },
    { id: 'A2', depth: 1 },
    { id: 'B', depth: 0 },
    { id: 'B1', depth: 1 },
  ]);
  assert.deepEqual(preorder(['A'], []), [{ id: 'A', depth: 0 }]);
  assert.deepEqual(preorder([], descendants), []);
});

test('windowOf: 코드 포인트 단위로 자르고, 끝을 넘는 offset은 빈 내용에 범위만 남긴다', () => {
  assert.deepEqual(windowOf('가나다라마', 0, 2), { content: '가나', range: { offset: 0, end: 2, total: 5 } });
  assert.deepEqual(windowOf('가나다라마', 3, 10), { content: '라마', range: { offset: 3, end: 5, total: 5 } });
  assert.deepEqual(windowOf('가나다라마', 5, 2), { content: '', range: { offset: 5, end: 5, total: 5 } });
  assert.deepEqual(windowOf('가나다라마', 9, 2), { content: '', range: { offset: 9, end: 5, total: 5 } });
  assert.deepEqual(windowOf('a😀b', 1, 1), { content: '😀', range: { offset: 1, end: 2, total: 3 } });
  assert.deepEqual(windowOf('', 0, 1), { content: '', range: { offset: 0, end: 0, total: 0 } });
});

test('입력 미러: create-folders', () => {
  assert.deepEqual(CreateFoldersInput.parse({ items: [{ name: '초고' }] }), { items: [{ name: '초고' }] });
  assert.deepEqual(CreateFoldersInput.parse({ items: [{ name: '초고', parentFolderId: 'FLDR0' }, { name: '퇴고' }] }), {
    items: [{ name: '초고', parentFolderId: 'FLDR0' }, { name: '퇴고' }],
  });
  assert.equal(CreateFoldersInput.safeParse({ items: [{ name: '' }] }).success, false);
  assert.equal(CreateFoldersInput.safeParse({ items: [{ name: '가'.repeat(101) }] }).success, false);
  assert.equal(CreateFoldersInput.safeParse({ items: [] }).success, false);
  assert.equal(CreateFoldersInput.safeParse({ items: Array.from({ length: 51 }, () => ({ name: '초고' })) }).success, false);
  assert.equal(CreateFoldersInput.safeParse({ name: '초고' }).success, false);
  assert.deepEqual(
    CreateFoldersInput.parse({
      items: [
        { name: '초고', after: 'DOC0' },
        { name: '퇴고', before: 'FLDR0', parentFolderId: 'FLDR1' },
      ],
    }),
    {
      items: [
        { name: '초고', after: 'DOC0' },
        { name: '퇴고', before: 'FLDR0', parentFolderId: 'FLDR1' },
      ],
    },
  );
});

test('입력 미러: delete-entities', () => {
  assert.deepEqual(DeleteEntitiesInput.parse({ ids: ['E1', 'E2'] }), { ids: ['E1', 'E2'] });
  assert.equal(DeleteEntitiesInput.safeParse({ ids: [] }).success, false);
  assert.equal(DeleteEntitiesInput.safeParse({ ids: Array.from({ length: 51 }, (_, i) => `E${i}`) }).success, false);
  assert.equal(DeleteEntitiesInput.safeParse({}).success, false);
});

test('snippetOf: 하이라이트 태그 제거·200자 절단', () => {
  assert.equal(snippetOf('<em>해변</em>의 아침'), '해변의 아침');
  assert.equal(snippetOf(undefined), null);
  assert.equal(snippetOf('가'.repeat(300))?.length, 200);
});

test('withinDays: 오늘 포함 최근 N일 달력 창 — 활동일 slice가 아니다', () => {
  const rows = ['2026-07-01', '2026-08-23', '2026-08-24'].map((d) => ({ date: dayjs.kst(d) }));
  const now = dayjs.kst('2026-08-24');
  assert.deepEqual(
    withinDays(rows, 2, now).map((r) => kstDate(r.date)),
    ['2026-08-23', '2026-08-24'],
  );
  assert.deepEqual(
    withinDays(rows, 30, now).map((r) => kstDate(r.date)),
    ['2026-08-23', '2026-08-24'],
  );
});

test('kstDate: KST 달력일 문자열', () => {
  assert.equal(kstDate(dayjs.kst('2026-08-24')), '2026-08-24');
});

test('notePreview: 200자 절단·개행 이후 낙거', () => {
  assert.equal(notePreview('첫 줄\n둘째 줄'), '첫 줄');
  assert.equal(notePreview('가'.repeat(300)).length, 200);
  assert.equal(notePreview(''), '');
});

test('pageOf: 상한까지만 담고 더 있으면 truncated로 알린다 — 휴지통 50·댓글 30', () => {
  assert.equal(TRASH_PAGE_SIZE, 50);
  assert.equal(COMMENT_PAGE_SIZE, 30);

  const rows = Array.from({ length: 51 }, (_, i) => `E${i}`);

  assert.deepEqual(pageOf(rows.slice(0, 3), TRASH_PAGE_SIZE), { items: ['E0', 'E1', 'E2'], truncated: false });
  assert.equal(pageOf(rows.slice(0, 50), TRASH_PAGE_SIZE).truncated, false);

  const trash = pageOf(rows, TRASH_PAGE_SIZE);
  assert.equal(trash.items.length, 50);
  assert.equal(trash.items.at(-1), 'E49');
  assert.equal(trash.truncated, true);

  assert.equal(pageOf(rows.slice(0, 30), COMMENT_PAGE_SIZE).truncated, false);

  const comments = pageOf(rows.slice(0, 31), COMMENT_PAGE_SIZE);
  assert.equal(comments.items.length, 30);
  assert.equal(comments.items.at(-1), 'E29');
  assert.equal(comments.truncated, true);
});

test('입력 미러: read-note·read-sharing·read-comments', () => {
  assert.deepEqual(ReadNoteInput.parse({ noteId: 'NOTE0' }), { noteId: 'NOTE0' });
  assert.equal(ReadNoteInput.safeParse({}).success, false);

  assert.deepEqual(ReadSharingInput.parse({ ids: ['E1'] }), { ids: ['E1'] });
  assert.equal(ReadSharingInput.safeParse({ ids: [] }).success, false);
  assert.equal(ReadSharingInput.safeParse({ ids: Array.from({ length: 21 }, (_, i) => `E${i}`) }).success, false);

  assert.deepEqual(ReadCommentsInput.parse({ id: 'DOC0' }), { id: 'DOC0', resolved: false });
  assert.deepEqual(ReadCommentsInput.parse({ id: 'DOC0', resolved: true }), { id: 'DOC0', resolved: true });
  assert.equal(ReadCommentsInput.safeParse({ id: 'DOC0', resolved: '아니오' }).success, false);
});

test('entityUrl: 와일드카드 서브도메인을 걷어낸 사용자 사이트 주소', () => {
  assert.equal(entityUrl('https://*.typie.me', 'abc'), 'https://typie.me/abc');
});

test('입력 미러: entity·document 계열', () => {
  assert.deepEqual(CreateDocumentsInput.parse({ items: [{}] }), { items: [{}] });
  assert.deepEqual(CreateDocumentsInput.parse({ items: [{ folderId: 'FLDR0' }, {}] }), { items: [{ folderId: 'FLDR0' }, {}] });
  assert.equal(CreateDocumentsInput.safeParse({}).success, false);
  assert.equal(CreateDocumentsInput.safeParse({ items: [] }).success, false);
  assert.deepEqual(CreateDocumentsInput.parse({ items: [{ after: 'DOC0' }, { before: 'DOC0', folderId: 'FLDR0' }] }), {
    items: [{ after: 'DOC0' }, { before: 'DOC0', folderId: 'FLDR0' }],
  });

  assert.deepEqual(UpdateFoldersInput.parse({ items: [{ folderId: 'FLDR0', name: '초고' }] }), {
    items: [{ folderId: 'FLDR0', name: '초고' }],
  });
  assert.equal(UpdateFoldersInput.safeParse({ items: [{ folderId: 'FLDR0', name: '' }] }).success, false);
  assert.equal(UpdateFoldersInput.safeParse({ items: [{ folderId: 'FLDR0', name: '가'.repeat(101) }] }).success, false);
  assert.equal(UpdateFoldersInput.safeParse({ items: [{ name: '초고' }] }).success, false);
  assert.equal(UpdateFoldersInput.safeParse({ folderId: 'FLDR0', name: '초고' }).success, false);

  assert.deepEqual(UpdateDocumentsInput.parse({ items: [{ documentId: 'DOC0', title: ' 바다 ' }] }), {
    items: [{ documentId: 'DOC0', title: '바다' }],
  });
  assert.deepEqual(UpdateDocumentsInput.parse({ items: [{ documentId: 'DOC0', title: '', subtitle: '부제' }] }), {
    items: [{ documentId: 'DOC0', title: null, subtitle: '부제' }],
  });
  assert.deepEqual(UpdateDocumentsInput.parse({ items: [{ documentId: 'DOC0', title: '' }] }), {
    items: [{ documentId: 'DOC0', title: null }],
  });
  assert.deepEqual(UpdateDocumentsInput.parse({ items: [{ documentId: 'DOC0', subtitle: ' '.repeat(3) }] }), {
    items: [{ documentId: 'DOC0', subtitle: null }],
  });
  assert.equal(UpdateDocumentsInput.safeParse({ items: [{ documentId: 'DOC0' }] }).success, true);
  assert.equal(UpdateDocumentsInput.safeParse({ items: [{ title: '바다' }] }).success, false);
  assert.equal(UpdateDocumentsInput.safeParse({ documentId: 'DOC0', title: '바다' }).success, false);

  assert.deepEqual(MoveEntitiesInput.parse({ ids: ['E1'] }), { ids: ['E1'] });
  assert.equal(MoveEntitiesInput.safeParse({ ids: [] }).success, false);
  assert.equal(MoveEntitiesInput.safeParse({ ids: Array.from({ length: 51 }, (_, i) => `E${i}`) }).success, false);
  assert.deepEqual(MoveEntitiesInput.parse({ ids: ['E1'], after: 'DOC0' }), { ids: ['E1'], after: 'DOC0' });
  assert.deepEqual(MoveEntitiesInput.parse({ ids: ['E1'], before: 'DOC0', folderId: 'FLDR0' }), {
    ids: ['E1'],
    before: 'DOC0',
    folderId: 'FLDR0',
  });

  assert.deepEqual(DuplicateDocumentsInput.parse({ ids: ['DOC0', 'DOC1'] }), { ids: ['DOC0', 'DOC1'] });
  assert.equal(DuplicateDocumentsInput.safeParse({ ids: [] }).success, false);
  assert.equal(DuplicateDocumentsInput.safeParse({ documentId: 'DOC0' }).success, false);

  assert.deepEqual(
    UpdateIconsInput.parse({
      items: [
        { id: 'E1', icon: 'star', iconColor: 'red' },
        { id: 'E2', icon: 'book' },
      ],
    }),
    {
      items: [
        { id: 'E1', icon: 'star', iconColor: 'red' },
        { id: 'E2', icon: 'book' },
      ],
    },
  );
  assert.deepEqual(UpdateIconsInput.parse({ items: [{ id: 'E1', iconColor: 'red' }] }), { items: [{ id: 'E1', iconColor: 'red' }] });
  assert.deepEqual(UpdateIconsInput.parse({ items: [{ id: 'E1' }] }), { items: [{ id: 'E1' }] });
  assert.equal(UpdateIconsInput.safeParse({ items: [{ icon: 'star' }] }).success, false);
  assert.equal(UpdateIconsInput.safeParse({ items: [] }).success, false);
  assert.equal(UpdateIconsInput.safeParse({ items: Array.from({ length: 51 }, (_, i) => ({ id: `E${i}`, icon: 'star' })) }).success, false);

  assert.deepEqual(RecoverEntitiesInput.parse({ ids: ['E1', 'D2'] }), { ids: ['E1', 'D2'] });
  assert.equal(RecoverEntitiesInput.safeParse({ ids: [] }).success, false);
  assert.equal(RecoverEntitiesInput.safeParse({ id: 'E1' }).success, false);
});

test('validIcon: 카탈로그 밖 이름·색 거절', () => {
  assert.equal(validIcon(ENTITY_ICON_NAMES[0], ENTITY_ICON_COLORS[0]), true);
  assert.equal(validIcon('not-an-icon', ENTITY_ICON_COLORS[0]), false);
  assert.equal(validIcon(ENTITY_ICON_NAMES[0], 'not-a-color'), false);
  assert.equal(validIcon(ENTITY_ICON_NAMES[0]), true);
  assert.equal(validIcon(undefined, ENTITY_ICON_COLORS[0]), true);
  assert.equal(validIcon('not-an-icon'), false);
  assert.equal(validIcon(undefined, 'not-a-color'), false);
});

test('입력 미러: note 계열', () => {
  assert.deepEqual(CreateNotesInput.parse({ items: [{ content: '메모' }] }), { items: [{ content: '메모' }] });
  assert.deepEqual(CreateNotesInput.parse({ items: [{ content: '메모', color: 'red' }, { content: '둘' }] }), {
    items: [{ content: '메모', color: 'red' }, { content: '둘' }],
  });
  assert.equal(CreateNotesInput.safeParse({ items: [{ content: '' }] }).success, false);
  assert.equal(CreateNotesInput.safeParse({ content: '메모' }).success, false);

  assert.deepEqual(UpdateNotesInput.parse({ items: [{ noteId: 'NOTE0' }] }), { items: [{ noteId: 'NOTE0' }] });
  assert.deepEqual(UpdateNotesInput.parse({ items: [{ noteId: 'NOTE0', status: 'RESOLVED' }] }), {
    items: [{ noteId: 'NOTE0', status: 'RESOLVED' }],
  });
  assert.equal(UpdateNotesInput.safeParse({ items: [{ noteId: 'NOTE0', content: '' }] }).success, false);
  assert.equal(UpdateNotesInput.safeParse({ items: [{ noteId: 'NOTE0', status: 'DONE' }] }).success, false);
  assert.equal(UpdateNotesInput.safeParse({ noteId: 'NOTE0' }).success, false);

  assert.deepEqual(NoteLinksInput.parse({ items: [{ noteId: 'NOTE0', id: 'E1' }] }), { items: [{ noteId: 'NOTE0', id: 'E1' }] });
  assert.equal(NoteLinksInput.safeParse({ items: [{ noteId: 'NOTE0' }] }).success, false);
  assert.equal(NoteLinksInput.safeParse({ noteId: 'NOTE0', id: 'E1' }).success, false);

  assert.deepEqual(DeleteNotesInput.parse({ noteIds: ['NOTE0', 'NOTE1'] }), { noteIds: ['NOTE0', 'NOTE1'] });
  assert.equal(DeleteNotesInput.safeParse({ noteIds: [] }).success, false);
  assert.equal(DeleteNotesInput.safeParse({ noteId: 'NOTE0' }).success, false);
});

test('입력 미러: goal 계열 — dueAt은 YYYY-MM-DD만', () => {
  assert.deepEqual(SetGoalsInput.parse({ items: [{ targetCharacterCount: 1000 }] }), { items: [{ targetCharacterCount: 1000 }] });
  assert.deepEqual(SetGoalsInput.parse({ items: [{ targetCharacterCount: 1000, id: 'E1', dueAt: '2026-08-24' }] }), {
    items: [{ targetCharacterCount: 1000, id: 'E1', dueAt: '2026-08-24' }],
  });
  assert.equal(SetGoalsInput.safeParse({ items: [{ targetCharacterCount: 0 }] }).success, false);
  assert.equal(SetGoalsInput.safeParse({ items: [{ targetCharacterCount: 1.5 }] }).success, false);
  assert.equal(SetGoalsInput.safeParse({ items: [{ targetCharacterCount: 1000, dueAt: '2026-08-24T00:00:00Z' }] }).success, false);
  assert.equal(SetGoalsInput.safeParse({ items: [{ targetCharacterCount: 1000, dueAt: '2026/08/24' }] }).success, false);
  assert.equal(SetGoalsInput.safeParse({ targetCharacterCount: 1000 }).success, false);

  assert.deepEqual(DeleteGoalsInput.parse({ items: [{}] }), { items: [{}] });
  assert.deepEqual(DeleteGoalsInput.parse({ items: [{ id: 'E1' }, {}] }), { items: [{ id: 'E1' }, {}] });
  assert.equal(DeleteGoalsInput.safeParse({}).success, false);
  assert.equal(DeleteGoalsInput.safeParse({ items: [] }).success, false);
});

test('kstDueDate: 달력에 없는 날짜는 무음 롤오버 대신 null', () => {
  assert.equal(kstDueDate('2026-08-24')?.format('YYYY-MM-DD'), '2026-08-24');
  assert.equal(kstDueDate('2024-02-29')?.format('YYYY-MM-DD'), '2024-02-29');
  assert.equal(kstDueDate('2026-02-31'), null);
  assert.equal(kstDueDate('2026-02-29'), null);
  assert.equal(kstDueDate('2026-04-31'), null);
  assert.equal(kstDueDate('2026-13-01'), null);
  assert.equal(kstDueDate('2026-00-10'), null);
});

test('kstDueDate: KST 자정을 가리킨다', () => {
  const due = kstDueDate('2026-08-24');
  assert.equal(due?.toISOString(), '2026-08-23T15:00:00.000Z');
});

test('입력 미러: update-sharing', () => {
  assert.deepEqual(UpdateSharingInput.parse({ ids: ['E1'], visibility: 'PRIVATE' }), {
    ids: ['E1'],
    visibility: 'PRIVATE',
  });
  assert.deepEqual(UpdateSharingInput.parse({ ids: ['E1'], visibility: 'PUBLIC', recursive: true }), {
    ids: ['E1'],
    visibility: 'PUBLIC',
    recursive: true,
  });
  assert.equal(UpdateSharingInput.safeParse({ ids: [], visibility: 'PUBLIC' }).success, false);
  assert.equal(UpdateSharingInput.safeParse({ ids: Array.from({ length: 21 }, (_, i) => `E${i}`), visibility: 'PUBLIC' }).success, false);
  assert.equal(UpdateSharingInput.safeParse({ ids: ['E1'], visibility: 'SECRET' }).success, false);
});

test('entityRefKind: TableCode 접두로 entity·document·folder를 가르고 그 밖은 null', () => {
  assert.equal(entityRefKind('E0ABCDEFGHIJ'), 'entity');
  assert.equal(entityRefKind('D0ABCDEFGHIJKLMN'), 'document');
  assert.equal(entityRefKind('F0ABCDEFGHIJ'), 'folder');
  assert.equal(entityRefKind('N0ABCDEFGHIJKLMN'), null);
  assert.equal(entityRefKind('EG0ABCDEFGHIJ'), null);
  assert.equal(entityRefKind(''), null);
});
