<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { Button, TextInput } from '@typie/ui/components';
  import { createForm } from '@typie/ui/form';
  import { z } from 'zod';
  import { getEditor } from '$lib/editor/context';

  type Props = {
    close: () => void;
  };

  let { close }: Props = $props();

  const editor = getEditor();

  const activeMarks = $derived(editor.activeMarks);
  const findMark = (type: string) => activeMarks.uniformMarks.find((m) => m.type === type);
  const isLinkActive = $derived(activeMarks.uniformMarks.some((m) => m.type === 'link'));
  const currentHref = $derived((findMark('link') as { href?: string })?.href ?? '');

  const form = createForm({
    schema: z.object({
      url: z.string().min(1),
    }),
    onSubmit: (data) => {
      const url = /^[^:]+:\/\//.test(data.url) ? data.url : `https://${data.url}`;
      editor.dispatch({ type: 'toggleLink', href: url });
      close();
    },
    defaultValues: {
      url: currentHref,
    },
  });

  $effect(() => {
    void form;
  });
</script>

<form class={flex({ alignItems: 'center', gap: '4px', padding: '4px' })} onsubmit={form.handleSubmit}>
  <TextInput autofocus placeholder="https://..." size="sm" bind:value={form.fields.url} />

  <Button size="sm" type="submit">{isLinkActive ? '수정' : '삽입'}</Button>

  {#if isLinkActive}
    <Button
      onclick={() => {
        editor.dispatch({ type: 'toggleLink', href: '' });
        close();
      }}
      size="sm"
      type="button"
      variant="secondary"
    >
      삭제
    </Button>
  {/if}
</form>
