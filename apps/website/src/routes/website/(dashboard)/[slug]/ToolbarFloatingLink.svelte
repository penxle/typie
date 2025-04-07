<script lang="ts">
  import { getTextBetween, isMarkActive } from '@tiptap/core';
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

  editor.current.chain().focus().extendMarkRange('link').run();

  const form = createForm({
    schema: z.object({
      url: z.string().min(1),
    }),
    onSubmit: (data) => {
      const url = /^[^:]+:\/\//.test(data.url) ? data.url : `https://${data.url}`;

      if (isMarkActive(editor.current.state, 'link')) {
        const text = getTextBetween(editor.current.state.doc, editor.current.state.selection);
        const existingUrl = editor.current.getAttributes('link').href;

        if (text === existingUrl || `https://${text}` === existingUrl) {
          editor.current
            .chain()
            .focus()
            .command(({ state, tr, dispatch }) => {
              const { from, to } = tr.selection;

              tr.replaceRangeWith(from, to, state.schema.text(url));
              tr.setSelection(TextSelection.create(tr.doc, from, from + url.length));
              tr.addMark(from, from + url.length, state.schema.mark('link', { href: url }));

              dispatch?.(tr);

              return true;
            })
            .run();
        } else {
          editor.current.chain().focus().updateLink(url).run();
        }
      } else {
        editor.current
          .chain()
          .focus()
          .command(({ state, tr, dispatch }) => {
            const { from, to, empty } = tr.selection;

            if (empty) {
              tr.replaceRangeWith(from, to, state.schema.text(url));
              tr.setSelection(TextSelection.create(tr.doc, from, from + url.length));
              tr.addMark(from, from + url.length, state.schema.mark('link', { href: url }));
            } else {
              tr.addMark(from, to, state.schema.mark('link', { href: url }));
            }

            dispatch?.(tr);

            return true;
          })
          .run();
      }

      close();
    },
    defaultValues: {
      url: editor.current.getAttributes('link').href,
    },
  });
</script>

<form
  class={flex({
    alignItems: 'center',
    gap: '4px',
    borderWidth: '1px',
    borderRadius: '4px',
    padding: '4px',
  })}
  onsubmit={form.handleSubmit}
>
  <TextInput autofocus placeholder="https://..." size="sm" bind:value={form.fields.url} />

  <Button size="sm" type="submit">삽입</Button>

  <!-- {#if editor.current.isActive('link')}
    <Button
      onclick={() => {
        editor.current.chain().focus().unsetLink().run();
        close();
      }}
      size="sm"
      type="button"
      variant="secondary"
    >
      삭제
    </Button>
  {/if} -->
</form>
