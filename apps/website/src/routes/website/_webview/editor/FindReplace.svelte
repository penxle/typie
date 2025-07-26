<script lang="ts">
  import { untrack } from 'svelte';
  import { createFindReplaceManager } from '$lib/editor/find-replace';
  import type { Editor } from '@tiptap/core';
  import type { FindReplaceManager } from '$lib/editor/find-replace';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
  };

  let { editor }: Props = $props();
  let manager: FindReplaceManager | null = $state(null);

  // Create manager when editor is available
  $effect(() => {
    if (editor?.current && !manager) {
      manager = createFindReplaceManager(editor.current);
    }
  });

  const searchHandler = async (data: { text: string }) => {
    if (!manager) return;

    const result = manager.search(data.text || '');
    return {
      totalMatches: result.results.length,
      currentMatch: result.currentIndex,
    };
  };

  const findNextHandler = async () => {
    if (!manager) return;
    const currentMatch = manager.next();
    return { currentMatch };
  };

  const findPreviousHandler = async () => {
    if (!manager) return;
    const currentMatch = manager.previous();
    return { currentMatch };
  };

  const replaceHandler = async (data: { replaceText: string }) => {
    if (!manager) return;

    const result = manager.replace(data.replaceText || '');
    return {
      success: result.success,
      currentMatch: result.currentIndex,
      totalMatches: manager.getResults().length,
    };
  };

  const replaceAllHandler = async (data: { findText: string; replaceText: string }) => {
    if (!manager || !data.findText) return;

    // Set search text first
    manager.search(data.findText);
    // Then replace all
    manager.replaceAll(data.replaceText || '');

    return {
      currentMatch: 0,
      totalMatches: 0,
    };
  };

  const clearSearchHighlightsHandler = async () => {
    if (!manager) return;
    manager.clear();
  };

  $effect(() => {
    return untrack(() => {
      window.__webview__?.setProcedure('search', searchHandler);
      window.__webview__?.setProcedure('findNext', findNextHandler);
      window.__webview__?.setProcedure('findPrevious', findPreviousHandler);
      window.__webview__?.setProcedure('replace', replaceHandler);
      window.__webview__?.setProcedure('replaceAll', replaceAllHandler);
      window.__webview__?.setProcedure('clearSearchHighlights', clearSearchHighlightsHandler);

      return () => {
        manager?.clear();
      };
    });
  });
</script>
