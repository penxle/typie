<script lang="ts">
  import { token } from '@typie/styled-system/tokens';
  import { getPriorityMeta } from './constants';
  import type { IssuePriority } from './constants';

  type Props = {
    priority: IssuePriority;
    size?: number;
  };

  let { priority, size = 14 }: Props = $props();

  const meta = $derived(getPriorityMeta(priority));
  const activeColor = token('colors.text.muted');
  const inactiveColor = token('colors.border.default');
</script>

<svg aria-label={meta.label} fill="none" height={size} viewBox="0 0 16 16" width={size}>
  {#if priority === 'URGENT'}
    <rect fill="#FF7A2E" height="16" rx="3" width="16" />
    <path d="M8 3.5V9" stroke="white" stroke-linecap="round" stroke-width="2" />
    <circle cx="8" cy="12" fill="white" r="1.2" />
  {:else if priority === 'NONE'}
    <path d="M2.5 8H4" stroke={activeColor} stroke-linecap="round" stroke-width="1.5" />
    <path d="M7.25 8H8.75" stroke={activeColor} stroke-linecap="round" stroke-width="1.5" />
    <path d="M12 8H13.5" stroke={activeColor} stroke-linecap="round" stroke-width="1.5" />
  {:else}
    {@const filledCount = priority === 'HIGH' ? 3 : priority === 'MEDIUM' ? 2 : 1}
    <rect fill={filledCount >= 1 ? activeColor : inactiveColor} height="5" rx="0.5" width="3" x="1" y="10" />
    <rect fill={filledCount >= 2 ? activeColor : inactiveColor} height="9" rx="0.5" width="3" x="6.5" y="6" />
    <rect fill={filledCount >= 3 ? activeColor : inactiveColor} height="14" rx="0.5" width="3" x="12" y="1" />
  {/if}
</svg>
