<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { onMount } from 'svelte';
  import * as Y from 'yjs';
  import { YState } from './state.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    doc: Y.Doc;
    editor?: Ref<Editor>;
  };

  let { doc, editor }: Props = $props();

  const anchors = new YState<Record<string, string | null>>(doc, 'anchors', {});

  const anchorElements = $derived.by(() => {
    if (!editor) {
      return {};
    }

    const elements: Record<string, HTMLElement> = {};

    for (const nodeId of Object.keys(anchors.current)) {
      const element = document.querySelector(`[data-node-id="${nodeId}"]`);
      if (element) {
        elements[nodeId] = element as HTMLElement;
      }
    }

    return elements;
  });

  const anchorPositions = $derived.by(() => {
    if (!editor || Object.keys(anchorElements).length === 0) return [];

    const editorEl = document.querySelector('.editor');
    if (!editorEl) return [];

    const totalHeight = editorEl.scrollHeight;
    if (totalHeight === 0) return [];

    return Object.entries(anchorElements)
      .map(([nodeId, element]) => {
        const offsetTop = element.offsetTop;
        const position = Math.min(1, Math.max(0, offsetTop / totalHeight));

        return {
          nodeId,
          position,
          name: anchors.current[nodeId] || element.textContent || '(앵커)',
        };
      })
      .sort((a, b) => a.position - b.position);
  });

  onMount(() => {
    window.__webview__?.setProcedure('getAnchorPositions', () => {
      return anchorPositions;
    });

    window.__webview__?.setProcedure('getCurrentNode', () => {
      if (!editor) return null;

      const { from } = editor.current.state.selection;
      const resolvedPos = editor.current.state.doc.resolve(from);

      let targetPos = from;
      targetPos = resolvedPos.before(2);

      const node = editor.current.state.doc.nodeAt(targetPos);

      if (node && node.attrs.nodeId) {
        const editorEl = document.querySelector('.editor');
        if (!editorEl) return null;

        const element = document.querySelector(`[data-node-id="${node.attrs.nodeId}"]`);
        if (!element) return null;

        const totalHeight = editorEl.scrollHeight;
        const offsetTop = (element as HTMLElement).offsetTop;
        const position = totalHeight > 0 ? Math.min(1, Math.max(0, offsetTop / totalHeight)) : 0;

        return {
          nodeId: node.attrs.nodeId,
          name: element.textContent || '(앵커)',
          position,
        };
      }

      return null;
    });

    window.__webview__?.setProcedure('clickAnchor', (nodeId: string) => {
      const element = document.querySelector(`[data-node-id="${nodeId}"]`);
      if (!editor || !element) return;

      const pos = editor.current.view.posAtDOM(element, 0);
      editor.current
        .chain()
        .setNodeSelection(pos - 1)
        .run();

      editor.current.commands.scrollIntoViewFixed({
        animate: true,
        position: 0.25,
      });

      mixpanel.track('anchor_click');
    });

    window.__webview__?.setProcedure('addAnchor', (nodeId: string) => {
      anchors.current = { ...anchors.current, [nodeId]: null };

      mixpanel.track('anchor_add');
    });

    window.__webview__?.setProcedure('removeAnchor', (nodeId: string) => {
      anchors.current = Object.fromEntries(Object.entries(anchors.current).filter(([key]) => key !== nodeId));

      mixpanel.track('anchor_remove');
    });
  });
</script>
