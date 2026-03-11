<script lang="ts">
  import { token } from '@typie/styled-system/tokens';
  import { getStatusMeta } from './constants';
  import type { IssueStatus } from './constants';

  type Props = {
    status: IssueStatus;
    size?: number;
  };

  let { status, size = 16 }: Props = $props();

  const meta = $derived(getStatusMeta(status));
  const color = $derived(token(`colors.${meta.colorToken}`));
</script>

<svg fill="none" height={size} viewBox="0 0 16 16" width={size}>
  {#if status === 'OPEN'}
    <circle
      cx="8"
      cy="8"
      pathLength="100"
      r="5.5"
      stroke={color}
      stroke-dasharray="2 6.33"
      stroke-dashoffset="4.17"
      stroke-linecap="round"
      stroke-width="1.5"
    />
  {:else if status === 'IN_PROGRESS'}
    <circle cx="8" cy="8" r="5.5" stroke={color} stroke-width="1.5" />
    <path d="M8 5A3 3 0 0 1 8 11V5Z" fill={color} stroke={color} stroke-linejoin="round" stroke-width="1" />
  {:else if status === 'RESOLVED'}
    <circle cx="8" cy="8" fill={color} r="6.5" />
    <path d="M5.5 8.5L7 10L10.5 6" stroke="white" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" />
  {:else if status === 'CLOSED'}
    <circle cx="8" cy="8" r="5.5" stroke={color} stroke-width="1.5" />
    <path d="M6 6L10 10M10 6L6 10" stroke={color} stroke-linecap="round" stroke-width="1.5" />
  {/if}
</svg>
