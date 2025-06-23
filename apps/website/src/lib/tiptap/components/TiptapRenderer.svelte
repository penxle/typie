<script lang="ts">
  import { Editor, Extension } from '@tiptap/core';
  import { onMount } from 'svelte';
  import { Ref } from '$lib/utils';
  import { css, cx } from '$styled-system/css';
  import { renderHTML } from '../lib/html';
  import { baseExtensions } from '../schema';
  import type { JSONContent } from '@tiptap/core';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    style?: SystemStyleObject;
    content: JSONContent;
    editor?: Ref<Editor>;
    extensions?: Extension[];
  };

  let { style, content, editor = $bindable(), extensions }: Props = $props();

  let element = $state<HTMLElement>();
  const html = $derived(renderHTML(content, [...baseExtensions, ...(extensions ?? [])]));

  onMount(() => {
    const e = new Editor({
      editable: false,
      content,
      extensions: [...baseExtensions, ...(extensions ?? [])],
      injectCSS: false,

      editorProps: {
        attributes: { class: css({ display: 'flex', flexDirection: 'column', alignItems: 'center' }, style) },
      },

      onCreate: ({ editor }) => {
        // eslint-disable-next-line svelte/no-dom-manipulating
        element?.replaceWith(editor.view.dom);
      },

      onTransaction: ({ editor: e }) => {
        editor = new Ref(e);
      },
    });

    editor = new Ref(e);

    return () => {
      editor?.current.destroy();
      editor = undefined;
    };
  });
</script>

<svelte:head>
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html html.head}
</svelte:head>

<article
  class={css({ display: 'contents', fontFamily: 'prose', whiteSpace: 'pre-wrap', overflowWrap: 'break-word', wordBreak: 'break-all' })}
>
  <div
    bind:this={element}
    class={cx('ProseMirror ProseMirror-hydratable', css({ display: 'flex', flexDirection: 'column', alignItems: 'center' }, style))}
  >
    <!-- eslint-disable-next-line svelte/no-unused-svelte-ignore -->
    <!-- svelte-ignore hydration_html_changed -->
    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
    {@html html.body}
  </div>
</article>
