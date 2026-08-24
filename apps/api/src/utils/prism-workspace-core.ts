// 순수 — env·DB·네트워크 import 없음(node:test 직접 로드)
import { ENTITY_ICON_COLORS, ENTITY_ICON_NAMES } from '@typie/lib/catalogs';
import { NoteStatus } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { z } from 'zod';
import { TableCode } from '#/db/schemas/codes.ts';
import { decodeDbId } from '#/db/schemas/id.ts';
import type { Dayjs } from 'dayjs';

export type EntityRefKind = 'entity' | 'document' | 'folder';

const ENTITY_REF_KINDS: Record<string, EntityRefKind | undefined> = {
  [TableCode.ENTITIES]: 'entity',
  [TableCode.DOCUMENTS]: 'document',
  [TableCode.FOLDERS]: 'folder',
};

export const entityRefKind = (id: string): EntityRefKind | null => ENTITY_REF_KINDS[decodeDbId(id)] ?? null;

export const SearchEntitiesInput = z.object({ query: z.string().min(1) });
export const ListEntitiesInput = z.object({ folderId: z.string().optional() });
export const DOCUMENT_WINDOW_DEFAULT = 2000;
export const DOCUMENT_WINDOW_MAX = 5000;
export const ReadDocumentInput = z.object({
  documentId: z.string(),
  offset: z.number().int().min(0).default(0),
  length: z.number().int().min(1).max(DOCUMENT_WINDOW_MAX).default(DOCUMENT_WINDOW_DEFAULT),
});
export const ReadNoteInput = z.object({ noteId: z.string() });
export const ReadSharingInput = z.object({ ids: z.array(z.string()).min(1).max(20) });
export const ReadCommentsInput = z.object({ documentId: z.string(), resolved: z.boolean().default(false) });
export const CreateFolderInput = z.object({ name: z.string().min(1).max(100), parentFolderId: z.string().optional() });
export const DeleteEntitiesInput = z.object({ ids: z.array(z.string()).min(1).max(50) });
export const CreateDocumentInput = z.object({ folderId: z.string().optional() });
export const RenameFolderInput = z.object({ folderId: z.string(), name: z.string().min(1).max(100) });
export const MoveEntitiesInput = z.object({ ids: z.array(z.string()).min(1).max(50), folderId: z.string().optional() });
export const DuplicateDocumentInput = z.object({ documentId: z.string() });
export const UpdateIconInput = z.object({ id: z.string(), icon: z.string(), iconColor: z.string() });
export const RecoverEntityInput = z.object({ id: z.string() });
export const CreateNoteInput = z.object({ content: z.string().min(1), color: z.string().optional() });
export const UpdateNoteInput = z.object({
  noteId: z.string(),
  content: z.string().min(1).optional(),
  color: z.string().optional(),
  status: z.enum(NoteStatus).optional(),
});
export const NoteLinkInput = z.object({ noteId: z.string(), id: z.string() });
export const SetGoalInput = z.object({
  targetCharacterCount: z.number().int().positive(),
  id: z.string().optional(),
  dueAt: z
    .string()
    .regex(/^\d{4}-\d{2}-\d{2}$/)
    .optional(),
});
export const DeleteNoteInput = z.object({ noteId: z.string() });
export const DeleteGoalInput = z.object({ id: z.string().optional() });
export const UpdateSharingInput = z.object({
  ids: z.array(z.string()).min(1).max(20),
  visibility: z.enum(['PUBLIC', 'UNLISTED', 'PRIVATE']),
  recursive: z.boolean().optional(),
});

export const validIcon = (icon: string, iconColor: string): boolean =>
  ENTITY_ICON_NAMES.includes(icon) && ENTITY_ICON_COLORS.includes(iconColor);

export type TextWindow = { content: string; range: { offset: number; end: number; total: number } };

export const windowOf = (text: string, offset: number, length: number): TextWindow => {
  const chars = [...text];
  const end = Math.min(offset + length, chars.length);

  return { content: offset >= chars.length ? '' : chars.slice(offset, end).join(''), range: { offset, end, total: chars.length } };
};

export const snippetOf = (highlighted: string | undefined): string | null =>
  highlighted === undefined ? null : highlighted.replaceAll(/<\/?em>/g, '').slice(0, 200);

export const notePreview = (content: string): string => (content.split('\n', 1)[0] ?? '').slice(0, 200);

export const TRASH_PAGE_SIZE = 50;
export const COMMENT_PAGE_SIZE = 30;

export const pageOf = <T>(rows: T[], size: number): { items: T[]; truncated: boolean } => ({
  items: rows.slice(0, size),
  truncated: rows.length > size,
});

export const entityUrl = (usersiteUrl: string, permalink: string): string => `${usersiteUrl.replace('*.', '')}/${permalink}`;

export const kstDate = (date: Dayjs): string => date.kst().format('YYYY-MM-DD');

export const kstDueDate = (date: string): Dayjs | null => {
  const parsed = dayjs.kst(date).startOf('day');

  return parsed.format('YYYY-MM-DD') === date ? parsed : null;
};

export const withinDays = <T extends { date: Dayjs }>(rows: T[], days: number, now: Dayjs): T[] => {
  const from = now
    .kst()
    .startOf('day')
    .subtract(days - 1, 'day');

  return rows.filter((row) => !row.date.isBefore(from));
};
