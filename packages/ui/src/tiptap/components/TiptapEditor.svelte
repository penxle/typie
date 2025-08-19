<script lang="ts">
  import { Editor, Extension } from '@tiptap/core';
  import { css } from '@typie/styled-system/css';
  import { getAllContexts, onMount } from 'svelte';
  import { Ref } from '../../utils';
  import { Collaboration } from '../extensions';
  import { baseExtensions, editorExtensions } from '../schema';
  import type { Storage } from '@tiptap/core';
  import type { EditorView } from '@tiptap/pm/view';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type * as YAwareness from 'y-protocols/awareness';
  import type * as Y from 'yjs';

  type Props = {
    style?: SystemStyleObject;
    editor?: Ref<Editor>;
    doc?: Y.Doc;
    awareness?: YAwareness.Awareness;
    undoManager?: Y.UndoManager;
    storage?: Partial<Storage>;
    extensions?: Extension[];
    editable?: boolean;
    onkeydown?: (view: EditorView, event: KeyboardEvent) => void;
    oncreate?: () => void;
    onfocus?: () => void;
    onblur?: () => void;
    onfile?: (event: { pos: number; file: File }) => void;
  };

  let {
    style,
    editor = $bindable(),
    doc,
    awareness,
    undoManager,
    storage,
    extensions,
    editable = true,
    onkeydown,
    oncreate,
    onfocus,
    onblur,
    onfile,
  }: Props = $props();

  let element = $state<HTMLDivElement>();
  const contexts = getAllContexts();

  onMount(() => {
    const e = new Editor({
      element,
      extensions: [
        ...baseExtensions,
        ...(editable ? editorExtensions : []),
        ...(extensions ?? []),
        ...(doc ? [Collaboration.configure({ doc, awareness, undoManager })] : []),
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

      onBeforeCreate: ({ editor }) => {
        editor.storage.contexts = contexts;
      },

      onCreate: () => {
        oncreate?.();
      },

      onFocus: () => {
        onfocus?.();
      },

      onBlur: () => {
        onblur?.();
      },
    });

    editor = new Ref(e);

    if (storage) {
      Object.assign(e.storage, storage);
    }

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
