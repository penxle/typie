<script lang="ts">
  import { onMount } from 'svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
  };

  let { editor }: Props = $props();

  onMount(() => {
    window.__webview__?.setProcedure('search', (text: string) => {
      editor?.current.commands.search(text);

      return {
        currentIndex: editor?.current.extensionStorage.search.currentIndex,
        totalCount: editor?.current.extensionStorage.search.matches.length,
      };
    });

    window.__webview__?.setProcedure('findNext', () => {
      editor?.current.commands.findNext();

      return {
        currentIndex: editor?.current.extensionStorage.search.currentIndex,
      };
    });

    window.__webview__?.setProcedure('findPrevious', () => {
      editor?.current.commands.findPrevious();

      return {
        currentIndex: editor?.current.extensionStorage.search.currentIndex,
      };
    });

    window.__webview__?.setProcedure('replace', (text: string) => {
      editor?.current.commands.replace(text);

      return {
        currentIndex: editor?.current.extensionStorage.search.currentIndex,
        totalCount: editor?.current.extensionStorage.search.matches.length,
      };
    });

    window.__webview__?.setProcedure('replaceAll', (text: string) => {
      editor?.current.commands.replaceAll(text);
    });

    window.__webview__?.setProcedure('clearSearch', () => {
      editor?.current.commands.clearSearch();
    });
  });
</script>
