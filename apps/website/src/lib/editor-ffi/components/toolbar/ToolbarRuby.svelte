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

  const existingRubyText = $derived(
    ctx.editor?.modifierState?.ruby?.type === 'uniform' ? ctx.editor.modifierState.ruby.value.text : undefined,
  );

  const form = createForm({
    schema: z.object({
      ruby: z.string().min(1),
    }),
    onSubmit: (data) => {
      const modifier: Modifier = { type: 'ruby', text: data.ruby };
      ctx.editor?.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: 'ruby', modifier } });
      ctx.editor?.focus();
      close();
    },
    defaultValues: {
      ruby: existingRubyText ?? '',
    },
  });
</script>

<form class={flex({ alignItems: 'center', gap: '4px', padding: '4px' })} onsubmit={form.handleSubmit}>
  <TextInput autofocus placeholder="텍스트 위에 들어갈 문구" size="sm" bind:value={form.fields.ruby} />

  <Button size="sm" type="submit">삽입</Button>

  {#if existingRubyText}
    <Button
      onclick={() => {
        ctx.editor?.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: 'ruby', modifier: undefined } });
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
