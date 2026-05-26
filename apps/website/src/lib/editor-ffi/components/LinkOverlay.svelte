<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '../editor.svelte';

  type Props = {
    page: number;
  };

  let { page }: Props = $props();

  const { editor } = getEditorContext();

  const links = $derived(editor?.linkRects.filter((r) => r.page_idx === page) ?? []);
  const interactive = $derived((editor?.readOnly ?? false) || (editor?.modifierHeld ?? false));

  const SAFE_PROTOCOLS = new Set(['http:', 'https:', 'mailto:', 'tel:']);
  const safeHref = (href: string): string => {
    try {
      const url = new URL(href, window.location.href);
      return SAFE_PROTOCOLS.has(url.protocol) ? href : '#';
    } catch {
      return '#';
    }
  };
</script>

{#each links as link (link.node_id)}
  {@const href = safeHref(link.href)}
  {#each link.rects as rect, i (i)}
    <a
      style:left="{rect.x}px"
      style:top="{rect.y}px"
      style:width="{rect.width}px"
      style:height="{rect.height}px"
      style:pointer-events={interactive ? 'auto' : 'none'}
      class={css({
        position: 'absolute',
        display: 'block',
        color: 'transparent',
        textDecoration: 'none',
        cursor: 'pointer',
      })}
      aria-label={link.href}
      {href}
      onpointerdown={(e) => e.stopPropagation()}
      rel="noopener noreferrer"
      tabindex={-1}
    ></a>
  {/each}
{/each}
