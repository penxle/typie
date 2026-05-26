import { goto } from '$app/navigation';
import type { NodeId } from '@typie/editor-ffi/browser';
import type { Editor } from '../editor.svelte';
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
          onclick: () => removeLink(editor, hit.link.node_id),
        },
      );
    }

    return items;
  };

  return editor.registerContextMenuContributor(contributor);
};

const normalizeUrl = (input: string): string =>
  /^https?:/i.test(input) || /^mailto:/i.test(input) || /^tel:/i.test(input) ? input : `https://${input}`;

const selectInsideNode = (editor: Editor, nodeId: NodeId): void => {
  const pos = { node_id: nodeId, offset: 0 };
  editor.enqueue({ type: 'selection', op: { type: 'set', selection: { anchor: pos, head: pos } } });
};

const editLinkPrompt = (editor: Editor, nodeId: NodeId, current: string): void => {
  const result = window.prompt('URL을 입력하세요 (비우고 확인을 누르면 제거)', current);
  if (result === null) {
    editor.focus();
    return;
  }
  const trimmed = result.trim();
  const modifier = trimmed === '' ? undefined : ({ type: 'link', href: normalizeUrl(trimmed) } as const);
  selectInsideNode(editor, nodeId);
  editor.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: 'link', modifier } });
  editor.focus();
};

const removeLink = (editor: Editor, nodeId: NodeId): void => {
  selectInsideNode(editor, nodeId);
  editor.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: 'link', modifier: undefined } });
  editor.focus();
};
