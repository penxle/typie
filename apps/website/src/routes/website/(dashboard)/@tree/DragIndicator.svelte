<script lang="ts" module>
  export type DragIndicatorState = Partial<{
    top: number;
    left: number;
    width: number;
    height: number;
    opacity: number;
    transform: string | undefined;
  }>;
</script>

<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { portal } from '@typie/ui/actions';
  import { fade } from 'svelte/transition';

  type Props = {
    indicator: DragIndicatorState;
  };

  let { indicator }: Props = $props();
</script>

{#key JSON.stringify(indicator)}
  <div
    style:top={`${indicator.top ?? -1}px`}
    style:left={`${indicator.left ?? -1}px`}
    style:width={`${indicator.width ?? 0}px`}
    style:height={`${indicator.height ?? 0}px`}
    style:opacity={indicator.opacity}
    style:transform={indicator.transform}
    class={css({
      position: 'fixed',
      borderRadius: '2px',
      backgroundColor: { base: 'accent.info.default/30', _dark: 'accent.info.default/40' },
      pointerEvents: 'none',
      zIndex: 'sidebar',
    })}
    use:portal
    transition:fade|global={{ duration: 100 }}
  ></div>
{/key}
