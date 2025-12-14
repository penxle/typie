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
  const isRubyActive = $derived(activeMarks.uniformMarks.some((m) => m.type === 'ruby'));
  const currentRubyText = $derived((findMark('ruby') as { text?: string })?.text ?? '');

  const form = createForm({
    schema: z.object({
      ruby: z.string().min(1),
    }),
    onSubmit: (data) => {
      editor.dispatch({ type: 'toggleRuby', text: data.ruby });
      close();
    },
    defaultValues: {
      ruby: currentRubyText,
    },
  });

  $effect(() => {
    void form;
  });
</script>

<form class={flex({ alignItems: 'center', gap: '4px', padding: '4px' })} onsubmit={form.handleSubmit}>
  <TextInput autofocus placeholder="텍스트 위에 들어갈 문구" size="sm" bind:value={form.fields.ruby} />

  <Button size="sm" type="submit">{isRubyActive ? '수정' : '삽입'}</Button>

  {#if isRubyActive}
    <Button
      onclick={() => {
        editor.dispatch({ type: 'toggleRuby', text: '' });
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
