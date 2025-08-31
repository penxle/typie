<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { getAppContext } from '@typie/ui/context';
  import { getViewContext } from '../@split-view/context.svelte';
  import type { AppPreference } from '@typie/ui/context';

  type Props = {
    tab: AppPreference['panelInfoTabByViewId'][string];
    label: string;
    badge?: number;
  };

  let { tab, label, badge }: Props = $props();

  const app = getAppContext();

  const splitViewId = getViewContext().id;
</script>

<button
  class={flex({
    alignItems: 'center',
    justifyContent: 'center',
    gap: '6px',
    borderRadius: '4px',
    paddingX: '12px',
    paddingY: '6px',
    flexGrow: '1',
    fontSize: '13px',
    fontWeight: 'semibold',
    color: 'text.subtle',
    _hover: { backgroundColor: 'surface.subtle' },
    _pressed: {
      color: 'text.default',
      backgroundColor: 'surface.muted',
    },
  })}
  aria-pressed={app.preference.current.panelInfoTabByViewId[splitViewId] === tab}
  onclick={() => {
    app.preference.current.panelInfoTabByViewId[splitViewId] = tab;
  }}
  type="button"
>
  {label}
  {#if badge}
    <div
      class={css({
        borderRadius: '4px',
        paddingX: '6px',
        paddingY: '2px',
        fontSize: '11px',
        fontWeight: 'semibold',
        color: 'text.default',
        backgroundColor: 'surface.subtle',
      })}
    >
      {badge}
    </div>
  {/if}
</button>
