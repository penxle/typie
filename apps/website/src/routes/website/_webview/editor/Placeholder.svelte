<script lang="ts">
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor: Ref<Editor>;
  };

  let { editor }: Props = $props();

  const isBodyEmpty = $derived.by(() => {
    const { doc, selection } = editor.current.state;
    const { empty } = selection;

    const body = doc.child(0);

    return (
      empty &&
      body.childCount === 1 &&
      body.child(0).type.name === 'paragraph' &&
      (body.child(0).attrs.textAlign === 'left' || body.child(0).attrs.textAlign === 'justify') &&
      body.child(0).childCount === 0
    );
  });

  const paragraphIndent = $derived.by(() => {
    const { doc } = editor.current.state;
    const body = doc.child(0);
    return body.attrs.paragraphIndent;
  });
</script>

{#if isBodyEmpty}
  <div class={center({ position: 'absolute', top: '0', insetX: '0', flexGrow: '1', pointerEvents: 'none' })}>
    <div
      style:padding-left={`${paragraphIndent}em`}
      class={flex({
        flexDirection: 'column',
        gap: '4px',
        width: 'full',
        maxWidth: 'var(--prosemirror-max-width)',
        color: 'gray.300',
        lineHeight: '[1.6]',
      })}
    >
      <div class={css({ fontFamily: 'ui' })}>내용</div>
    </div>
  </div>
{/if}
