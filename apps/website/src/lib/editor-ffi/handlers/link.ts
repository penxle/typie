import { tick } from 'svelte';
import { goto } from '$app/navigation';
import type { Editor, EditorContext } from '../editor.svelte';
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

type LinkPoint = { page: number; x: number; y: number };

type LinkEditorTarget = Pick<Editor, 'enqueue' | 'flush' | 'focus' | 'modifierSpanSelection' | 'selection'>;

// A link is an inline modifier (no single node id), so target it by location:
// place a caret at the page-local point, then expand to the covering link span.
const selectLinkSpanAtPoint = (editor: LinkEditorTarget, point: LinkPoint): void => {
  editor.enqueue({ type: 'selection', op: { type: 'set_at', page: point.page, x: point.x, y: point.y } });
  editor.flush();
  const caret = editor.selection?.head;
  if (!caret) return;
  const selection = editor.modifierSpanSelection(caret, 'link') ?? { anchor: caret, head: caret };
  editor.enqueue({ type: 'selection', op: { type: 'set', selection } });
  editor.flush();
};

export const registerLinkContextMenu = (editor: Editor): (() => void) => {
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
    if (!editor.readOnly && local) {
      const point: LinkPoint = { page: local.page, x: local.x, y: local.y };
      items.push(
        {
          label: '링크 편집',
          onclick: () => editLinkPrompt(editor, point, hit.link.href),
        },
        {
          label: '링크 제거',
          variant: 'danger',
          onclick: () => removeLinkAtPoint(editor, point),
        },
      );
    }

    return items;
  };

  return editor.registerContextMenuContributor(contributor);
};

const editLinkAtPoint = (editor: Editor, point: LinkPoint, href: string): void => {
  selectLinkSpanAtPoint(editor, point);
  editor.enqueue({
    type: 'modifier',
    op: { type: 'edit', modifier_type: 'link', modifier: { type: 'link', href: normalizeUrl(href.trim()) } },
  });
  editor.focus();
};

const removeLinkAtPoint = (editor: Editor, point: LinkPoint): void => {
  selectLinkSpanAtPoint(editor, point);
  editor.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: 'link', modifier: undefined } });
  editor.focus();
};

const editLinkPrompt = (editor: Editor, point: LinkPoint, current: string): void => {
  const result = window.prompt('URL을 입력하세요 (비우고 확인을 누르면 제거)', current);
  if (result === null) {
    editor.focus();
    return;
  }
  const trimmed = result.trim();
  if (trimmed === '') {
    removeLinkAtPoint(editor, point);
  } else {
    editLinkAtPoint(editor, point, trimmed);
  }
};

type LinkEditorContextLike = Pick<EditorContext, 'linkEditorOpen'>;

type OpenLinkEditorFromTooltipOptions = {
  closeTooltip: () => void;
  ctx: LinkEditorContextLike;
  editor: LinkEditorTarget | undefined;
  point: LinkPoint;
};

// Opens the toolbar link editor from the hover tooltip's "edit" action.
export const openLinkEditorFromTooltip = async ({
  closeTooltip,
  ctx,
  editor,
  point,
}: OpenLinkEditorFromTooltipOptions): Promise<boolean> => {
  if (!editor) return false;

  // Extend the selection over the whole link span so editing/removal applies to
  // the entire mark, not just the caret position.
  selectLinkSpanAtPoint(editor, point);
  editor.focus();

  closeTooltip();

  // If the editor is already open (e.g. switching links), force it to re-read
  // the freshly extended selection by closing it for a tick before reopening.
  if (ctx.linkEditorOpen) {
    ctx.linkEditorOpen = false;
    await tick();
  }
  ctx.linkEditorOpen = true;
  return true;
};
