<script lang="ts">
  import { TextSelection } from '@tiptap/pm/state';
  import { z } from 'zod';
  import { Button, TextInput } from '$lib/components';
  import { createForm } from '$lib/form';
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
    }),
    onSubmit: (data) => {
      editor.current
        .chain()
        .focus()
        .command(({ state, tr, dispatch }) => {
          const { from, to, empty } = tr.selection;

          if (empty) {
            const base = '하단 텍스트';
            tr.replaceRangeWith(from, to, state.schema.text(base));
            tr.setSelection(TextSelection.create(tr.doc, from, from + base.length));
            tr.addMark(from, from + base.length, state.schema.mark('ruby', { text: data.ruby }));
          } else {
            tr.addMark(from, to, state.schema.mark('ruby', { text: data.ruby }));
          }

          dispatch?.(tr);

          return true;
        })
        .run();

      close();
    },
    defaultValues: {
      ruby: editor.current.getAttributes('ruby').text,
    },
  });
</script>

<form class={flex({ alignItems: 'center', gap: '4px', padding: '4px' })} onsubmit={form.handleSubmit}>
  <TextInput autofocus placeholder="텍스트 위에 들어갈 문구" size="sm" bind:value={form.fields.ruby} />

  <Button size="sm" type="submit">삽입</Button>

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
      제거
    </Button>
  {/if}
</form>
