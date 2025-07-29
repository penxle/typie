<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { onMount } from 'svelte';
  import * as Y from 'yjs';
  import { clamp } from '$lib/utils';
  import { YState } from './state.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    doc: Y.Doc;
    editor?: Ref<Editor>;
  };

  let { doc, editor }: Props = $props();

  const anchors = new YState<Record<string, string | null>>(doc, 'anchors', {});

  const getLastNodeOffsetTop = () => {
    const editorEl = document.querySelector('.editor');
    if (!editorEl) return null;

    const allNodes = [...editorEl.querySelectorAll('[data-node-id]')];
    if (allNodes.length === 0) return null;

    const lastNode = allNodes.at(-1) as HTMLElement;
    return lastNode.offsetTop;
  };

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

    const lastNodeOffsetTop = getLastNodeOffsetTop();
    if (!lastNodeOffsetTop) return [];

    return Object.entries(anchorElements)
      .map(([nodeId, element]) => {
        const offsetTop = element.offsetTop;
        const position = lastNodeOffsetTop > 0 ? clamp(offsetTop / lastNodeOffsetTop, 0, 1) : 0;

        return {
          nodeId,
          position,
          name: anchors.current[nodeId] || element.textContent || '(내용 없음)',
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
        const element = document.querySelector(`[data-node-id="${node.attrs.nodeId}"]`);
        if (!element) return null;

        const lastNodeOffsetTop = getLastNodeOffsetTop();
        if (!lastNodeOffsetTop) return null;

        const offsetTop = (element as HTMLElement).offsetTop;
        const position = lastNodeOffsetTop > 0 ? clamp(offsetTop / lastNodeOffsetTop, 0, 1) : 0;

        return {
          nodeId: node.attrs.nodeId,
          name: element.textContent || '(내용 없음)',
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

    window.__webview__?.setProcedure('scrollToTop', () => {
      if (!editor) return;

      editor.current.chain().setTextSelection(1).run();

      editor.current.commands.scrollIntoViewFixed({
        animate: true,
        position: 0.25,
      });

      mixpanel.track('anchor_scroll_to_top');
    });

    window.__webview__?.setProcedure('scrollToBottom', () => {
      if (!editor) return;

      const endPos = editor.current.state.doc.content.size - 2;
      editor.current.chain().setTextSelection(endPos).run();

      editor.current.commands.scrollIntoViewFixed({
        animate: true,
        position: 0.25,
      });

      mixpanel.track('anchor_scroll_to_bottom');
    });
  });
</script>
