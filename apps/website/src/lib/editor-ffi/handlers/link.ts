import { goto } from '$lib/navigation';
import type { CommandOutcome, PageRect } from '@typie/editor-ffi/browser';
import type { Editor, EditorContext, EditorSnapshot } from '../editor.svelte';
import type { EditorRequest } from '../editor-update';
import type { ContextMenuContributor, ContextMenuItem } from '../types';

const SAFE_PROTOCOLS = new Set(['http:', 'https:', 'mailto:', 'tel:']);

export const openLink = (href: string): void => {
  let url: URL | null;
  try {
    url = new URL(href, window.location.href);
  } catch {
    url = null;
  }

  if (!url || !SAFE_PROTOCOLS.has(url.protocol)) return;

  if (url.origin === window.location.origin) {
    void goto(url.pathname + url.search + url.hash);
    return;
  }

  window.open(url.href, '_blank', 'noopener,noreferrer');
};

export const normalizeUrl = (input: string): string =>
  /^https?:/i.test(input) || /^mailto:/i.test(input) || /^tel:/i.test(input) ? input : `https://${input}`;

export type LinkPoint = { page: number; x: number; y: number };

type LinkEditorTarget = Pick<Editor, 'enqueue' | 'focus' | 'modifierSpanSelection'> & {
  updateNow: (
    build: (request: EditorRequest) => void,
  ) => { snapshot: Pick<EditorSnapshot, 'selection'>; commandOutcomes: readonly CommandOutcome[] } | null;
};

// A link is an inline modifier (no single node id), so target it by location:
// place a caret at the page-local point, then expand to the covering link span.
export const selectLinkSpanAtPoint = (editor: LinkEditorTarget, point: LinkPoint): void => {
  const update = editor.updateNow(() =>
    editor.enqueue({ type: 'selection', op: { type: 'set_at', page: point.page, x: point.x, y: point.y } }),
  );
  if (!update || update.commandOutcomes.some((outcome) => outcome.type === 'rejected')) return;
  const caret = update.snapshot.selection?.head;
  if (!caret) return;
  const selection = editor.modifierSpanSelection(caret, 'link') ?? { anchor: caret, head: caret };
  editor.updateNow(() => editor.enqueue({ type: 'selection', op: { type: 'set', selection } }));
};

export const removeLinkAtPoint = (editor: LinkEditorTarget, point: LinkPoint): void => {
  selectLinkSpanAtPoint(editor, point);
  editor.updateNow(() => editor.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: 'link', modifier: undefined } }));
  editor.focus();
};

type RegisterLinkContextMenuOptions = {
  onEdit?: (point: LinkPoint, anchor: PageRect) => void;
};

export const registerLinkContextMenu = (editor: Editor, options: RegisterLinkContextMenuOptions = {}): (() => void) => {
  const contributor: ContextMenuContributor = ({ clientX, clientY }) => {
    const hit = editor.linkHitTestAtClient(clientX, clientY);
    if (!hit) return [];

    const items: ContextMenuItem[] = [
      {
        label: '링크 열기',
        onclick: () => openLink(hit.link.href),
      },
    ];

    const local = editor.clientToLocal(clientX, clientY);
    if (local && !editor.readOnly) {
      const point: LinkPoint = { page: local.page, x: local.x, y: local.y };
      const { onEdit } = options;
      const rect = hit.link.rects[0];
      if (onEdit && rect) {
        items.push({
          label: '링크 편집',
          onclick: () => onEdit(point, { page_idx: hit.page, rect }),
        });
      }
      items.push({
        label: '링크 제거',
        variant: 'danger',
        onclick: () => removeLinkAtPoint(editor, point),
      });
    }

    return items;
  };

  return editor.registerContextMenuContributor(contributor);
};

type OpenLinkCardAtPointOptions = {
  ctx: Pick<EditorContext, 'markCard'>;
  editor: LinkEditorTarget | undefined;
  point: LinkPoint;
  anchor: PageRect;
};

export const openLinkCardAtPoint = ({ ctx, editor, point, anchor }: OpenLinkCardAtPointOptions): boolean => {
  if (!editor) return false;

  selectLinkSpanAtPoint(editor, point);
  editor.focus();
  ctx.markCard = { kind: 'link', mode: 'edit', anchor };
  return true;
};
