<script lang="ts">
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { normalizeUrl } from '../../handlers/link';
  import LinkEditorForm from './LinkEditorForm.svelte';
  import type { Modifier } from '@typie/editor-ffi/browser';

  type Props = {
    close: () => void;
  };

  let { close }: Props = $props();

  const ctx = getEditorContext();

  const existingHref = $derived(ctx.editor?.modifierState?.link?.type === 'uniform' ? ctx.editor.modifierState.link.value.href : undefined);
</script>

<LinkEditorForm
  {close}
  initialHref={existingHref}
  onRemove={existingHref
    ? () => {
        ctx.editor?.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: 'link', modifier: undefined } });
        ctx.editor?.focus();
      }
    : undefined}
  onSubmit={(href) => {
    const modifier: Modifier = { type: 'link', href: normalizeUrl(href) };
    ctx.editor?.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: 'link', modifier } });
    ctx.editor?.focus();
  }}
/>
