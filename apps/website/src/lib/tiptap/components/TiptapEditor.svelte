<script lang="ts">
  import { Editor, Extension } from '@tiptap/core';
  import { onMount } from 'svelte';
  import { Ref } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { Collaboration } from '../extensions';
  import { baseExtensions, editorExtensions } from '../schema';
  import type { EditorView } from '@tiptap/pm/view';
  import type * as YAwareness from 'y-protocols/awareness';
  import type * as Y from 'yjs';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    style?: SystemStyleObject;
    editor?: Ref<Editor>;
    doc?: Y.Doc;
    awareness?: YAwareness.Awareness;
    extensions?: Extension[];
    editable?: boolean;
    onkeydown?: (view: EditorView, event: KeyboardEvent) => void;
    oncreate?: () => void;
    onfile?: (event: { pos: number; file: File }) => void;
  };

  let { style, editor = $bindable(), doc, awareness, extensions, editable = true, onkeydown, oncreate, onfile }: Props = $props();

  let element = $state<HTMLDivElement>();

  onMount(() => {
    const e = new Editor({
      element,
      extensions: [
        ...baseExtensions,
        ...(editable ? editorExtensions : []),
        ...(extensions ?? []),
        ...(doc ? [Collaboration.configure({ doc, awareness })] : []),
      ],
      injectCSS: false,
      autofocus: false,
      editable,

      editorProps: {
        attributes: {
          class: css({ display: 'flex', flexDirection: 'column', alignItems: 'center' }, style),
          autocorrect: 'off',
          autocapitalize: 'off',
          spellcheck: 'false',
        },

        scrollMargin: window.__webview__ ? 24 : 48,
        scrollThreshold: window.__webview__ ? 24 : 48,

        handleKeyDown: onkeydown,

        clipboardTextSerializer: (content) => {
          const text = content.content.textBetween(0, content.content.size, '\n', (node) => {
            if (node.type.name === 'hard_break') {
              return '\n';
            }

            return '';
          });
          return text.replaceAll(/\n{3,}/g, '\n\n');
        },

        handleDrop: (view, event) => {
          if (event.dataTransfer?.files?.length) {
            const pos = view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos ?? view.state.selection.anchor;
            for (const file of event.dataTransfer.files) {
              onfile?.({ pos, file });
            }

            return true;
          }

          return false;
        },

        handlePaste: (view, event) => {
          if (event.clipboardData?.files?.length) {
            const pos = view.state.selection.anchor;
            for (const file of event.clipboardData.files) {
              onfile?.({ pos, file });
            }

            return true;
          }

          return false;
        },
      },

      onTransaction: ({ editor: e }) => {
        if ($effect.tracking()) {
          setTimeout(() => {
            editor = new Ref(e);
          }, 0);
        } else {
          editor = new Ref(e);
        }
      },

      onCreate: () => {
        oncreate?.();
      },
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
