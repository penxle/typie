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
      ruby: z.string().min(1),
    }),
    onSubmit: (data) => {
      editor.focus().dispatch({ type: 'addAnnotation', annotation: { type: 'ruby', text: data.ruby } });
      close();
    },
    defaultValues: {
      ruby: '',
    },
  });
</script>

<form class={flex({ alignItems: 'center', gap: '4px', padding: '4px' })} onsubmit={form.handleSubmit}>
  <TextInput autofocus placeholder="텍스트 위에 들어갈 문구" size="sm" bind:value={form.fields.ruby} />

  <Button size="sm" type="submit">삽입</Button>
</form>
