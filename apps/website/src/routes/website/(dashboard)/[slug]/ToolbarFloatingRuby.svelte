<script lang="ts">
  import { getTextBetween } from '@tiptap/core';
  import { TextSelection } from '@tiptap/pm/state';
  import { z } from 'zod';
  import { Button, TextInput } from '$lib/components';
  import { createForm } from '$lib/form';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor: Ref<Editor>;
    close: () => void;
  };

  let { editor, close }: Props = $props();

  editor.current.chain().focus().extendMarkRange('ruby').run();

  const form = createForm({
    schema: z.object({
      ruby: z.string().min(1),
      base: z.string().min(1),
    }),
    onSubmit: (data) => {
      editor.current
        .chain()
        .focus()
        .command(({ state, tr, dispatch }) => {
          const { from, to } = tr.selection;

          tr.replaceRangeWith(from, to, state.schema.text(data.base));
          tr.setSelection(TextSelection.create(tr.doc, from, from + data.base.length));
          tr.addMark(from, from + data.base.length, state.schema.mark('ruby', { text: data.ruby }));

          dispatch?.(tr);

          return true;
        })
        .run();

      close();
    },
    defaultValues: {
      ruby: editor.current.getAttributes('ruby').text,
      base: getTextBetween(editor.current.state.doc, editor.current.state.selection),
    },
  });
</script>

<form
  class={flex({
    flexDirection: 'column',
    alignItems: 'center',
    gap: '12px',
    borderWidth: '1px',
    borderRadius: '4px',
    padding: '12px',
  })}
  onsubmit={form.handleSubmit}
>
  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <div class={css({ fontSize: '12px', color: 'gray.500' })}>루비 텍스트</div>
    <TextInput size="sm" bind:value={form.fields.ruby} />
  </div>

  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <div class={css({ fontSize: '12px', color: 'gray.500' })}>하단 텍스트</div>
    <TextInput size="sm" bind:value={form.fields.base} />
  </div>

  <div class={flex({ justifyContent: 'space-between', width: 'full' })}>
    {#if editor.current.isActive('ruby')}
      <Button
        onclick={() => {
          editor.current.chain().focus().unsetRuby().run();
          close();
        }}
        size="sm"
        type="button"
        variant="secondary"
      >
        삭제
      </Button>
    {:else}
      <div></div>
    {/if}

    <Button size="sm" type="submit">삽입</Button>
  </div>
</form>
