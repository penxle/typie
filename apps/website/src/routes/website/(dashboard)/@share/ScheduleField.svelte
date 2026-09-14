<script lang="ts">
  import '@typie/lib/dayjs';

  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Popover } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import { nextSchedulableParts } from '$lib/publication/publish-form';
  import SchedulePicker from './SchedulePicker.svelte';
  import type { ScheduleParts } from '$lib/publication/publish-form';

  type Props = {
    parts: ScheduleParts;
  };

  let { parts = $bindable() }: Props = $props();

  const label = $derived(`${dayjs(parts.date).format('YYYY. M. D. (ddd)')} ${parts.time}`);

  const pullIntoFuture = () => {
    parts = nextSchedulableParts(parts, dayjs());
  };
</script>

<Popover
  style={css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    width: 'full',
    height: '32px',
    paddingX: '10px',
    borderWidth: '1px',
    borderRadius: '6px',
    fontSize: '12px',
    color: 'text.default',
    fontVariantNumeric: 'tabular-nums',
    transition: 'common',
    _hover: { borderColor: 'border.emphasis' },
    _expanded: { borderColor: 'border.emphasis' },
  })}
  contentStyle={css.raw({ padding: '10px' })}
  offset={4}
  onopen={pullIntoFuture}
  placement="bottom-start"
>
  {#snippet trigger()}
    <span>{label}</span>
    <Icon style={css.raw({ marginLeft: 'auto', color: 'text.hint' })} icon={ChevronDownIcon} size={14} />
  {/snippet}

  <div class={flex({ minWidth: '0' })}>
    <SchedulePicker bind:parts />
  </div>
</Popover>
