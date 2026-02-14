<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { Button, TextInput } from '@typie/ui/components';
  import { createForm } from '@typie/ui/form';
  import { z } from 'zod';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import type { LinkAnnotationValue } from '$lib/editor/types';

  type Props = {
    close: () => void;
  };

  let { close }: Props = $props();

  const { editor } = getEditorContext();

  const linkAttr = editor.getAttr('link');
  const existingLink = linkAttr?.values.find((v): v is LinkAnnotationValue => v != null);

  const form = createForm({
    schema: z.object({
      url: z.string().min(1),
    }),
    onSubmit: (data) => {
      const url = /^[^:]+:\/\//.test(data.url) ? data.url : `https://${data.url}`;
      const type = existingLink ? 'updateAnnotation' : 'addAnnotation';
      editor.focus().dispatch({ type, annotation: { type: 'link', href: url } });
      close();
    },
    defaultValues: {
      url: existingLink?.href ?? '',
    },
  });
</script>

<form class={flex({ alignItems: 'center', gap: '4px', padding: '4px' })} onsubmit={form.handleSubmit}>
  <TextInput autofocus placeholder="https://..." size="sm" bind:value={form.fields.url} />

  <Button size="sm" type="submit">삽입</Button>

  {#if existingLink}
    <Button
      onclick={() => {
        editor.focus().dispatch({ type: 'removeAnnotation', annotationType: 'link' });
        close();
      }}
      size="sm"
      variant="secondary"
    >
      제거
    </Button>
  {/if}
</form>
