<script lang="ts">
  import { Editor } from '@tiptap/core';
  import { onMount } from 'svelte';
  import { css } from '$styled-system/css';
  import { Collaboration } from '../extensions/collaboration';
  import { extensions } from '../schema';
  import type * as YAwareness from 'y-protocols/awareness';
  import type * as Y from 'yjs';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    style?: SystemStyleObject;
    editor?: Editor;
    doc?: Y.Doc;
    awareness?: YAwareness.Awareness;
  };

  let { style, editor = $bindable(), doc, awareness }: Props = $props();

  let element = $state<HTMLDivElement>();

  onMount(() => {
    editor = new Editor({
      element,
      extensions: [...extensions, ...(doc && awareness ? [Collaboration.configure({ doc, awareness })] : [])],
      injectCSS: false,
      editorProps: {
        attributes: { class: css(style) },
        scrollMargin: { top: 150, bottom: 50, left: 0, right: 0 },
        scrollThreshold: { top: 150, bottom: 50, left: 0, right: 0 },
        handleKeyDown: (_, event) => {
          // 맥 구름입력기에서 엔터키 입력시 마지막 글자 잘리는 문제 workaround
          if (editor && event.key === 'Enter') {
            const s = editor.view.state.selection;
            editor.commands.setTextSelection(s.to);
          }
        },
      },
    });

    return () => {
      editor?.destroy();
      editor = undefined;
    };
  });
</script>

<div
  bind:this={element}
  class={css({ display: 'contents', fontFamily: 'prose', whiteSpace: 'pre-wrap', wordBreak: 'break-all' })}
  autocapitalize="off"
  spellcheck="false"
></div>
