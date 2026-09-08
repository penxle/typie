<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import CheckIcon from '~icons/lucide/check';
  import { FEATURES } from './pricing';

  const listClass = css({ display: 'grid', rowGap: '10px' });
  const itemClass = css({
    display: 'flex',
    alignItems: 'flex-start',
    gap: '10px',
    fontFamily: 'landing',
    fontSize: '14px',
    lineHeight: '[1.6]',
    color: 'text.muted',
  });
</script>

<ul class={listClass}>
  {#each FEATURES as feature (feature)}
    <li class={itemClass}>
      <Icon style={css.raw({ flexShrink: '0', marginTop: '5px', color: 'text.default' })} icon={CheckIcon} size={14} />
      {feature}
    </li>
  {/each}
</ul>
