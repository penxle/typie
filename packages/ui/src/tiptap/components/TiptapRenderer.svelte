<script lang="ts">
  import { Editor, Extension } from '@tiptap/core';
  import { css, cx } from '@typie/styled-system/css';
  import { onMount } from 'svelte';
  import { Ref } from '../../utils';
  import { renderHTML } from '../lib/html';
  import { baseExtensions } from '../schema';
  import type { JSONContent } from '@tiptap/core';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { PageLayout } from '../extensions';

  type Props = {
    style?: SystemStyleObject;
    content: JSONContent;
    editor?: Ref<Editor>;
    extensions?: Extension[];
    pageLayout?: PageLayout;
    forPdf?: boolean;
  };

  let { style, content, editor = $bindable(), extensions, pageLayout, forPdf }: Props = $props();

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
        if (pageLayout) {
          editor.commands.setPageLayout(pageLayout, forPdf ?? false);
        }
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
  style:--prosemirror-max-width={pageLayout ? `${pageLayout.width}mm` : '100%'}
  style:--prosemirror-page-margin-top={pageLayout ? `${pageLayout.marginTop}mm` : '0'}
  style:--prosemirror-page-margin-bottom={pageLayout ? `${pageLayout.marginBottom}mm` : '0'}
  style:--prosemirror-page-margin-left={pageLayout ? `${pageLayout.marginLeft}mm` : '0'}
  style:--prosemirror-page-margin-right={pageLayout ? `${pageLayout.marginRight}mm` : '0'}
  class={css({ display: 'contents', fontFamily: 'prose', whiteSpace: 'pre-wrap', overflowWrap: 'break-word', wordBreak: 'break-all' })}
  data-layout={pageLayout ? 'page' : 'scroll'}
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
