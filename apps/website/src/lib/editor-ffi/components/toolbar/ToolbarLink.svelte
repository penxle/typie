<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { Button, TextInput } from '@typie/ui/components';
  import { createForm } from '@typie/ui/form';
  import { z } from 'zod';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import type { Modifier } from '@typie/editor-ffi/browser';

  type Props = {
    close: () => void;
  };

  let { close }: Props = $props();

  const ctx = getEditorContext();

  const existingHref = $derived(ctx.editor?.modifierState?.link?.type === 'uniform' ? ctx.editor.modifierState.link.value.href : undefined);

  const normalizeUrl = (input: string): string => (/^[a-z][a-z0-9+.-]*:/i.test(input) ? input : `https://${input}`);

  const form = createForm({
    schema: z.object({
      url: z.string().min(1),
    }),
    onSubmit: (data) => {
      const href = normalizeUrl(data.url.trim());
      const modifier: Modifier = { type: 'link', href };
      ctx.editor?.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: 'link', modifier } });
      ctx.editor?.focus();
      close();
    },
    defaultValues: {
      url: existingHref ?? '',
    },
  });
</script>

<form class={flex({ alignItems: 'center', gap: '4px', padding: '4px' })} onsubmit={form.handleSubmit}>
  <TextInput autofocus placeholder="https://..." size="sm" bind:value={form.fields.url} />

  <Button size="sm" type="submit">삽입</Button>

  {#if existingHref}
    <Button
      onclick={() => {
        ctx.editor?.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: 'link', modifier: undefined } });
        ctx.editor?.focus();
        close();
      }}
      size="sm"
      variant="secondary"
    >
      제거
    </Button>
  {/if}
</form>
