<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import CheckIcon from '~icons/lucide/check';
  import CopyIcon from '~icons/lucide/copy';
  import LinkIcon from '~icons/lucide/link';
  import SendIcon from '~icons/lucide/send';
  import PropertyRow from './PropertyRow.svelte';
  import { propertyHintStyle, propertyNoteStyle, propertyTextStyle } from './publish-styles';

  type Props = {
    counts: { unpublished: number; scheduled: number; published: number };
    urls: string[];
    modifiedScheduled: number;
    modifiedPublished: number;
    scheduledLinkShared: boolean | null;
  };

  let { counts, urls, modifiedScheduled, modifiedPublished, scheduledLinkShared }: Props = $props();

  const segments = $derived(
    (
      [
        { kind: 'published', label: '발행됨', count: counts.published },
        { kind: 'scheduled', label: '예약됨', count: counts.scheduled },
        { kind: 'unpublished', label: '미발행', count: counts.unpublished },
      ] as const
    ).filter((segment) => segment.count > 0),
  );

  const notes = $derived(
    [
      scheduledLinkShared !== null && counts.scheduled > 0 && counts.published === 0 && counts.unpublished === 0
        ? scheduledLinkShared
          ? '예약 시각까지는 링크가 있는 누구나 볼 수 있어요.'
          : '예약 시각까지는 나만 볼 수 있어요.'
        : null,
      modifiedScheduled > 0 ? `예약 후 수정된 내용이 있는 글 ${modifiedScheduled}개는 예약에 반영해야 함께 올라가요.` : null,
      modifiedPublished > 0 ? `발행 후 수정된 내용이 있는 글 ${modifiedPublished}개가 있어요` : null,
    ].filter((note): note is string => note !== null),
  );

  let copied = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    return () => {
      if (timer) clearTimeout(timer);
    };
  });

  const copy = async () => {
    await navigator.clipboard.writeText(urls.join('\n'));
    if (timer) clearTimeout(timer);
    copied = true;
    timer = setTimeout(() => (copied = false), 2000);
  };
</script>

<div
  class={flex({
    position: 'relative',
    flexDirection: 'column',
    gap: '1px',
    marginX: '-8px',
    paddingBottom: '10px',
    _after: {
      content: '""',
      position: 'absolute',
      left: '8px',
      right: '8px',
      bottom: '0',
      height: '1px',
      backgroundColor: 'border.hairline',
    },
  })}
>
  <PropertyRow icon={SendIcon} label="발행 상태">
    <span class={flex({ alignItems: 'center', gap: '10px', minHeight: '30px', paddingX: '8px', marginLeft: '-8px', fontSize: '13px' })}>
      {#each segments as segment, index (segment.kind)}
        {#if index > 0}
          <span class={css(propertyHintStyle)}>·</span>
        {/if}

        <span class={flex({ alignItems: 'center', gap: '6px' })}>
          {#if segment.kind === 'published'}
            <span class={css({ flexShrink: '0', size: '8px', borderRadius: 'full', backgroundColor: 'success.default' })}></span>
          {:else if segment.kind === 'scheduled'}
            <span class={css({ flexShrink: '0', size: '8px', borderRadius: 'full', backgroundColor: 'warning.default' })}></span>
          {/if}
          <span class={css(propertyTextStyle, { fontVariantNumeric: 'tabular-nums' })}>{segment.label} {segment.count}</span>
        </span>
      {/each}
    </span>
  </PropertyRow>

  {#if urls.length > 0}
    <PropertyRow icon={LinkIcon} label="주소">
      <Button style={css.raw({ gap: '6px' })} onclick={copy} size="sm" variant="secondary">
        <Icon style={copied ? css.raw({ color: 'success.default' }) : undefined} icon={copied ? CheckIcon : CopyIcon} size={14} />
        {copied ? '복사되었어요' : `발행 주소 ${urls.length}개 복사`}
      </Button>
    </PropertyRow>
  {/if}

  {#each notes as note (note)}
    <p class={css(propertyNoteStyle)}>{note}</p>
  {/each}
</div>
