<script lang="ts">
  import {
    calculateAnchorPositions,
    cleanOrphanAnchors,
    findAnchorableNode,
    getAnchorElements,
    getLastNodeOffsetTop,
  } from '@typie/ui/anchor';
  import { clamp } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { onMount } from 'svelte';
  import * as Y from 'yjs';
  import { YState } from './state.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';

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

    return getAnchorElements(editor.current, Object.keys(anchors.current));
  });

  const anchorPositions = $derived.by(() => {
    if (!editor || Object.keys(anchorElements).length === 0) return [];

    return calculateAnchorPositions(editor.current, anchorElements, anchors.current).map(({ nodeId, position, name, excerpt }) => ({
      nodeId,
      position,
      name,
      excerpt,
    }));
  });

  onMount(() => {
    // @deprecated
    window.__webview__?.setProcedure('getAnchorPositions', () => {
      return anchorPositions.map(({ nodeId, position, name, excerpt }) => ({
        nodeId,
        position,
        name: name ?? excerpt,
      }));
    });

    window.__webview__?.setProcedure('getAnchorPositionsV2', () => {
      return anchorPositions;
    });

    // @deprecated
    window.__webview__?.setProcedure('getCurrentNode', () => {
      if (!editor) return null;

      const { from } = editor.current.state.selection;
      const resolvedPos = editor.current.state.doc.resolve(from);

      let targetPos = from;
      targetPos = resolvedPos.before(2);

      const node = editor.current.state.doc.nodeAt(targetPos);

      if (node && node.attrs.nodeId) {
        const element = editor.current.view.dom.querySelector(`[data-node-id="${node.attrs.nodeId}"]`);
        if (!element) return null;

        const lastNodeOffsetTop = getLastNodeOffsetTop(editor.current.view.dom);
        if (lastNodeOffsetTop === null) return null;

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

    window.__webview__?.setProcedure('getCurrentNodeV2', () => {
      if (!editor) return null;

      const { nodeId } = findAnchorableNode(editor.current);

      if (!nodeId) return null;

      const element = editor.current.view.dom.querySelector(`[data-node-id="${nodeId}"]`);
      if (!element) return null;

      const lastNodeOffsetTop = getLastNodeOffsetTop(editor.current.view.dom);
      if (lastNodeOffsetTop === null) return null;

      const offsetTop = (element as HTMLElement).offsetTop;
      const position = lastNodeOffsetTop > 0 ? clamp(offsetTop / lastNodeOffsetTop, 0, 1) : 0;

      return {
        nodeId,
        name: null,
        excerpt: element.textContent
          ? element.textContent.length > 20
            ? element.textContent.slice(0, 20) + '...'
            : element.textContent
          : '(내용 없음)',
        position,
      };
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

    window.__webview__?.setProcedure('addAnchorWithName', ({ nodeId, name }: { nodeId: string; name: string | null }) => {
      anchors.current = { ...anchors.current, [nodeId]: name };

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

    window.__webview__?.setProcedure('updateAnchorName', ({ nodeId, name }: { nodeId: string; name: string }) => {
      const trimmedName = name.trim();
      const finalName = trimmedName || null;

      anchors.current = { ...anchors.current, [nodeId]: finalName };

      if (finalName) {
        mixpanel.track('anchor_rename');
      } else {
        mixpanel.track('anchor_reset');
      }
    });

    if (editor?.current) {
      cleanOrphanAnchors(editor.current, doc);
    }
  });
</script>
