<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import CheckIcon from '~icons/lucide/check';
  import ClockIcon from '~icons/lucide/clock';
  import CopyIcon from '~icons/lucide/copy';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import LinkIcon from '~icons/lucide/link';
  import SendIcon from '~icons/lucide/send';
  import { formatPublishTime } from '$lib/publication/publish-form';
  import PropertyRow from './PropertyRow.svelte';
  import {
    linkFieldButtonStyle,
    linkFieldInputStyle,
    linkFieldStyle,
    modifiedBadgeStyle,
    propertyHintStyle,
    propertyTextStyle,
  } from './publish-styles';

  type Props = {
    publicationState: 'PUBLISHED' | 'SCHEDULED';
    publishedAt: string | null;
    scheduledAt: string | null;
    modified: boolean;
    url: string;
    linkShared: boolean;
  };

  let { publicationState, publishedAt, scheduledAt, modified, url, linkShared }: Props = $props();

  const scheduled = $derived(publicationState === 'SCHEDULED');
  const at = $derived(scheduled ? scheduledAt : publishedAt);
  const time = $derived(at ? formatPublishTime(at, dayjs()) : null);

  const note = $derived(
    scheduled
      ? [
          linkShared ? '예약 시각까지는 링크가 있는 누구나 볼 수 있어요.' : '예약 시각까지는 나만 볼 수 있어요.',
          modified ? '예약 후 수정된 내용이 있어요. 예약에 반영해야 함께 올라가요.' : null,
        ]
          .filter(Boolean)
          .join(' ')
      : modified
        ? '발행 후 수정된 내용이 있어요'
        : null,
  );

  let copied = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    return () => {
      if (timer) clearTimeout(timer);
    };
  });

  const copy = async () => {
    await navigator.clipboard.writeText(url);
    if (timer) clearTimeout(timer);
    copied = true;
    timer = setTimeout(() => (copied = false), 2000);
  };

  const selectUrl = (event: Event) => {
    (event.currentTarget as HTMLInputElement).select();
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
  <PropertyRow icon={scheduled ? ClockIcon : SendIcon} label="발행 상태">
    <span class={flex({ alignItems: 'center', gap: '8px', minHeight: '30px', paddingX: '8px', marginLeft: '-8px', fontSize: '13px' })}>
      <span
        class={css({
          flexShrink: '0',
          size: '8px',
          borderRadius: 'full',
          backgroundColor: scheduled ? 'warning.default' : 'success.default',
        })}
      ></span>

      <span class={css(propertyTextStyle)}>{scheduled ? '예약됨' : '발행됨'}</span>

      {#if modified}
        <span class={css(modifiedBadgeStyle)}>수정됨</span>
      {/if}

      {#if time}
        <span class={css(propertyHintStyle, { fontVariantNumeric: 'tabular-nums' })}>{time}</span>
      {/if}
    </span>
  </PropertyRow>

  {#if !scheduled}
    <PropertyRow icon={LinkIcon} label="주소">
      <span class={css(linkFieldStyle)}>
        <input
          class={css(linkFieldInputStyle)}
          aria-label="발행된 글 주소"
          autocomplete="off"
          onclick={selectUrl}
          onfocus={selectUrl}
          readonly
          spellcheck="false"
          value={url}
        />

        <button
          class={center(linkFieldButtonStyle)}
          aria-label="링크 복사"
          onclick={copy}
          type="button"
          use:tooltip={{ message: copied ? '복사되었어요' : '링크 복사', placement: 'top', keepOnClick: true }}
        >
          <Icon style={copied ? css.raw({ color: 'success.default' }) : undefined} icon={copied ? CheckIcon : CopyIcon} size={14} />
        </button>

        <a
          class={center(linkFieldButtonStyle)}
          aria-label="글 보기"
          href={url}
          rel="noopener noreferrer"
          target="_blank"
          use:tooltip={{ message: '글 보기', placement: 'top' }}
        >
          <Icon icon={ExternalLinkIcon} size={14} />
        </a>
      </span>
    </PropertyRow>
  {/if}

  {#if note}
    <p
      class={css({
        marginTop: '-4px',
        paddingLeft: '132px',
        paddingRight: '8px',
        fontSize: '12px',
        lineHeight: '[1.5]',
        color: 'text.hint',
      })}
    >
      {note}
    </p>
  {/if}
</div>
