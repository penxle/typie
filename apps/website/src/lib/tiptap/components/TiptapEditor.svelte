<script lang="ts">
  import { Editor } from '@tiptap/core';
  import { onMount } from 'svelte';
  import { Ref } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { Collaboration } from '../extensions/collaboration';
  import { extensions } from '../schema';
  import type { EditorView } from '@tiptap/pm/view';
  import type * as YAwareness from 'y-protocols/awareness';
  import type * as Y from 'yjs';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    style?: SystemStyleObject;
    editor?: Ref<Editor>;
    doc?: Y.Doc;
    awareness?: YAwareness.Awareness;
    onkeydown?: (view: EditorView, event: KeyboardEvent) => void;
    oncreate?: () => void;
  };

  let { style, editor = $bindable(), doc, awareness, onkeydown, oncreate }: Props = $props();

  let element = $state<HTMLDivElement>();

  onMount(() => {
    const e = new Editor({
      element,
      extensions: [...extensions, ...(doc && awareness ? [Collaboration.configure({ doc, awareness })] : [])],
      injectCSS: false,

      editorProps: {
        attributes: { class: css({ display: 'flex', flexDirection: 'column', alignItems: 'center' }, style) },
        scrollMargin: { top: 250, bottom: 150, left: 0, right: 0 },
        scrollThreshold: { top: 250, bottom: 150, left: 0, right: 0 },

        handleKeyDown: onkeydown,
      },

      onTransaction: ({ editor: e }) => {
        editor = new Ref(e);
      },

      onCreate: oncreate,
    });

    editor = new Ref(e);

    return () => {
      editor?.current.destroy();
      editor = undefined;
    };
  });
</script>

<div
  bind:this={element}
  class={css({ display: 'contents', fontFamily: 'prose', whiteSpace: 'pre-wrap', overflowWrap: 'break-word', wordBreak: 'break-all' })}
  autocapitalize="off"
  spellcheck="false"
></div>
