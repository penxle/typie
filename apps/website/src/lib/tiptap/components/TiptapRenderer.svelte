<script lang="ts">
  import { Editor } from '@tiptap/core';
  import { onMount } from 'svelte';
  import { css, cx } from '$styled-system/css';
  import { renderHTML } from '../lib/html';
  import { extensions } from '../schema';
  import type { JSONContent } from '@tiptap/core';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    style?: SystemStyleObject;
    content: JSONContent;
    editor?: Editor;
  };

  let { style, content, editor = $bindable() }: Props = $props();

  let element = $state<HTMLElement>();
  const html = $derived(renderHTML(content, extensions));

  onMount(() => {
    editor = new Editor({
      editable: false,
      content,
      extensions,
      injectCSS: false,

      editorProps: {
        attributes: { class: css(style) },
      },

      onCreate: ({ editor }) => {
        // eslint-disable-next-line svelte/no-dom-manipulating
        element?.replaceWith(editor.view.dom);
      },
    });

    return () => {
      editor?.destroy();
      editor = undefined;
    };
  });
</script>

<svelte:head>
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html html.head}
</svelte:head>

<article
  class={css({
    display: 'contents',
    fontFamily: 'prose',
    whiteSpace: 'pre-wrap',
    overflowWrap: 'break-word',
    wordBreak: 'break-all',
  })}
>
  <div bind:this={element} class={cx('ProseMirror', css(style))}>
    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
    {@html html.body}
  </div>
</article>
