<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { Button, TextInput } from '@typie/ui/components';

  type Props = {
    close: () => void;
    initialHref?: string;
    submitLabel?: string;
    onSubmit: (href: string) => void;
    onRemove?: () => void;
  };

  let { close, initialHref, submitLabel = '삽입', onSubmit, onRemove }: Props = $props();

  let url = $state(initialHref ?? '');

  $effect(() => {
    url = initialHref ?? '';
  });
</script>

<form
  class={flex({ alignItems: 'center', gap: '4px', padding: '4px' })}
  onsubmit={(event) => {
    event.preventDefault();
    const trimmed = url.trim();
    if (!trimmed) return;
    onSubmit(trimmed);
    close();
  }}
>
  <TextInput autofocus placeholder="https://..." size="sm" bind:value={url} />

  <Button disabled={url.trim() === ''} size="sm" type="submit">
    {submitLabel}
  </Button>

  {#if onRemove}
    <Button
      onclick={() => {
        onRemove();
        close();
      }}
      size="sm"
      variant="secondary"
    >
      제거
    </Button>
  {/if}
</form>
