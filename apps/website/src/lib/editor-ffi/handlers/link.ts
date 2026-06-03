import { tick } from 'svelte';
import { goto } from '$app/navigation';
import type { NodeId } from '@typie/editor-ffi/browser';
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

    if (!editor.readOnly) {
      items.push(
        {
          label: '링크 편집',
          onclick: () => editLinkPrompt(editor, hit.link.node_id, hit.link.href),
        },
        {
          label: '링크 제거',
          variant: 'danger',
          onclick: () => removeLinkAtNode(editor, hit.link.node_id),
        },
      );
    }

    return items;
  };

  return editor.registerContextMenuContributor(contributor);
};

const selectInsideNode = (editor: Editor, nodeId: NodeId): void => {
  const pos = { node_id: nodeId, offset: 0 };
  editor.enqueue({ type: 'selection', op: { type: 'set', selection: { anchor: pos, head: pos } } });
};

const editLinkAtNode = (editor: Editor, nodeId: NodeId, href: string): void => {
  selectInsideNode(editor, nodeId);
  editor.enqueue({
    type: 'modifier',
    op: { type: 'edit', modifier_type: 'link', modifier: { type: 'link', href: normalizeUrl(href.trim()) } },
  });
  editor.focus();
};

const removeLinkAtNode = (editor: Editor, nodeId: NodeId): void => {
  selectInsideNode(editor, nodeId);
  editor.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: 'link', modifier: undefined } });
  editor.focus();
};

const editLinkPrompt = (editor: Editor, nodeId: NodeId, current: string): void => {
  const result = window.prompt('URL을 입력하세요 (비우고 확인을 누르면 제거)', current);
  if (result === null) {
    editor.focus();
    return;
  }
  const trimmed = result.trim();
  if (trimmed === '') {
    removeLinkAtNode(editor, nodeId);
  } else {
    editLinkAtNode(editor, nodeId, trimmed);
  }
};

type LinkEditorContextLike = Pick<EditorContext, 'linkEditorOpen'>;
type LinkEditorTarget = Pick<Editor, 'enqueue' | 'flush' | 'focus' | 'modifierSpanSelection'>;

type OpenLinkEditorFromTooltipOptions = {
  closeTooltip: () => void;
  ctx: LinkEditorContextLike;
  editor: LinkEditorTarget | undefined;
  nodeId: NodeId;
};

// Opens the toolbar link editor from the hover tooltip's "edit" action.
export const openLinkEditorFromTooltip = async ({
  closeTooltip,
  ctx,
  editor,
  nodeId,
}: OpenLinkEditorFromTooltipOptions): Promise<boolean> => {
  if (!editor) return false;

  // Extend the selection over the whole link span so editing/removal applies to
  // the entire mark, not just the caret position. Fall back to a collapsed caret
  // inside the node if the span cannot be resolved.
  const caret = { node_id: nodeId, offset: 0 };
  const selection = editor.modifierSpanSelection(caret, 'link') ?? { anchor: caret, head: caret };
  editor.enqueue({ type: 'selection', op: { type: 'set', selection } });
  editor.flush();
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
