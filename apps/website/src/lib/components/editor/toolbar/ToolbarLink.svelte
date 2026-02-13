<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { Button, TextInput } from '@typie/ui/components';
  import { createForm } from '@typie/ui/form';
  import { z } from 'zod';
  import { getEditorContext } from '$lib/editor/context.svelte';

  type Props = {
    close: () => void;
  };

  let { close }: Props = $props();

  const { editor } = getEditorContext();

  const form = createForm({
    schema: z.object({
      url: z.string().min(1),
    }),
    onSubmit: (data) => {
      const url = /^[^:]+:\/\//.test(data.url) ? data.url : `https://${data.url}`;
      editor.focus().dispatch({ type: 'addAnnotation', annotation: { type: 'link', href: url } });
      close();
    },
    defaultValues: {
      url: '',
    },
  });
</script>

<form class={flex({ alignItems: 'center', gap: '4px', padding: '4px' })} onsubmit={form.handleSubmit}>
  <TextInput autofocus placeholder="https://..." size="sm" bind:value={form.fields.url} />

  <Button size="sm" type="submit">삽입</Button>
</form>
