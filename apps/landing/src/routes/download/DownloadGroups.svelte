<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import ArrowUpRightIcon from '~icons/lucide/arrow-up-right';
  import DownloadIcon from '~icons/lucide/download';
  import { DESKTOP_GROUP, MOBILE_GROUP, WEB_GROUP } from './download';
  import type { Download, Group } from './download';

  const actionIcon = (item: Download) => (item.id === 'web' ? ArrowRightIcon : item.external ? ArrowUpRightIcon : DownloadIcon);

  const columnsClass = css({
    display: 'grid',
    gridTemplateColumns: { base: '1fr', lg: '[repeat(2, minmax(0, 1fr))]' },
    alignItems: 'start',
    gap: { base: '40px', lg: '24px' },
  });
  const stackClass = css({ display: 'grid', gap: '40px' });
  const headClass = css({
    display: 'flex',
    alignItems: 'baseline',
    justifyContent: 'space-between',
    gap: '16px',
    paddingX: '4px',
    marginBottom: '12px',
  });
  const titleClass = css({ fontSize: '15px', fontWeight: 'semibold', color: 'text.default' });
  const noteClass = css({ fontSize: '12px', color: 'text.hint' });
  const listClass = css({ borderRadius: '[16px]', borderWidth: '1px', borderColor: 'border.hairline', overflow: 'hidden' });
  const rowClass = cx(
    'group',
    css({
      display: 'flex',
      alignItems: 'center',
      gap: '14px',
      paddingX: { base: '18px', md: '22px' },
      paddingY: '18px',
      color: 'text.default',
      transition: '[background-color 0.2s ease-out]',
      _hover: { backgroundColor: 'text.default/4' },
      '& + &': { borderTopWidth: '1px', borderTopColor: 'border.hairline' },
    }),
  );
  const nameClass = css({ fontSize: '15px', fontWeight: 'semibold' });
  const detailClass = css({ fontFamily: 'mono', fontSize: '12px', color: 'text.hint' });
  const actionStyle = css.raw({
    color: 'text.muted',
    transition: '[color 0.2s ease-out, transform 0.2s ease-out]',
    _groupHover: { color: 'text.default' },
  });
  const webActionStyle = css.raw({
    color: 'text.muted',
    transition: '[color 0.2s ease-out, transform 0.2s ease-out]',
    _groupHover: { color: 'text.default', transform: 'translateX(2px)' },
  });
</script>

{#snippet group(data: Group)}
  <section>
    <div class={headClass}>
      <h2 class={titleClass}>{data.title}</h2>
      {#if data.note}<span class={noteClass}>{data.note}</span>{/if}
    </div>
    <div class={listClass}>
      {#each data.items as item (item.id)}
        <a
          class={rowClass}
          href={item.url}
          rel={item.external ? 'noopener noreferrer' : undefined}
          target={item.external ? '_blank' : undefined}
        >
          <Icon style={css.raw({ flexShrink: '0' })} icon={item.icon} size={20} />
          <span class={nameClass}>{item.platform}</span>
          <span class={detailClass}>{item.detail}</span>
          <span class={css({ flex: '1' })}></span>
          <Icon style={item.id === 'web' ? webActionStyle : actionStyle} icon={actionIcon(item)} size={18} />
        </a>
      {/each}
    </div>
  </section>
{/snippet}

<div class={columnsClass}>
  <div class={stackClass}>
    {@render group(DESKTOP_GROUP)}
    {@render group(WEB_GROUP)}
  </div>
  {@render group(MOBILE_GROUP)}
</div>
