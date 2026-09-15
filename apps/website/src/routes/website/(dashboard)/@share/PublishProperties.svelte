<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon, Popover } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ClockIcon from '~icons/lucide/clock';
  import ImageIcon from '~icons/lucide/image';
  import TagIcon from '~icons/lucide/tag';
  import TextIcon from '~icons/lucide/text';
  import XIcon from '~icons/lucide/x';
  import { LoadableImg } from '$lib/components';
  import { composeScheduledAt, formatPublishTime } from '$lib/publication/publish-form';
  import { uploadBlobAsImage } from '$lib/utils';
  import PropertyRow from './PropertyRow.svelte';
  import { propertyChevronStyle, propertyNoteStyle, propertyTriggerStyle } from './publish-styles';
  import ScheduleField from './ScheduleField.svelte';
  import TagPills from './TagPills.svelte';
  import type { ModifiedPublishFields, PublishMode, ScheduleParts } from '$lib/publication/publish-form';

  type Props = {
    autoExcerpt: string;
    tags: string[];
    excerpt: string;
    thumbnailId: string | null;
    mode: PublishMode | 'keep';
    schedule: ScheduleParts;
    modified: ModifiedPublishFields;
    scheduleEditable: boolean;
    disabled?: boolean;
    partialTags?: { tag: string; count: number }[];
    onremovepartialtag?: (tag: string) => void;
    showMeta?: boolean;
    metaHint?: string | null;
  };

  let {
    autoExcerpt,
    tags = $bindable(),
    excerpt = $bindable(),
    thumbnailId = $bindable(),
    mode = $bindable(),
    schedule = $bindable(),
    modified,
    scheduleEditable,
    disabled = false,
    partialTags = [],
    onremovepartialtag,
    showMeta = true,
    metaHint = null,
  }: Props = $props();

  const scheduleLabel = $derived(formatPublishTime(composeScheduledAt(schedule), dayjs()));

  let thumbnailUploading = $state(false);

  let excerptOpen = $state(false);
  let excerptEl = $state<HTMLTextAreaElement>();

  $effect(() => {
    if (!excerptOpen) return;

    const el = excerptEl;
    if (!el) return;

    const frame = requestAnimationFrame(() => el.focus());
    return () => cancelAnimationFrame(frame);
  });

  const handleThumbnailUpload = () => {
    const input = window.document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';

    input.addEventListener('change', async () => {
      const file = input.files?.[0];
      if (!file) return;

      thumbnailUploading = true;
      try {
        const image = await uploadBlobAsImage(file);
        thumbnailId = image.id;
      } finally {
        thumbnailUploading = false;
      }
    });

    input.click();
  };
</script>

{#snippet whenOption(value: PublishMode, label: string, description: string)}
  {@const selected = mode === value}

  <button
    class={flex({
      alignItems: 'center',
      gap: '10px',
      padding: '8px',
      borderRadius: '6px',
      textAlign: 'left',
      transition: 'common',
      _hover: { backgroundColor: 'surface.hover' },
    })}
    aria-checked={selected}
    onclick={() => (mode = value)}
    role="radio"
    type="button"
  >
    <span class={flex({ flexDirection: 'column', gap: '1px', flex: '1', minWidth: '0' })}>
      <span class={css({ fontSize: '13px', fontWeight: 'medium' })}>{label}</span>
      <span class={css({ fontSize: '12px', color: 'text.hint' })}>{description}</span>
    </span>

    <span
      class={center({
        flexShrink: '0',
        size: '16px',
        borderWidth: '[1.5px]',
        borderColor: selected ? 'accent.default' : 'border.emphasis',
        borderRadius: 'full',
        transition: 'common',
      })}
    >
      {#if selected}
        <span class={css({ size: '8px', borderRadius: 'full', backgroundColor: 'accent.default' })}></span>
      {/if}
    </span>
  </button>
{/snippet}

<div class={flex({ flexDirection: 'column', gap: '1px', marginX: '-8px' })}>
  <PropertyRow icon={TagIcon} label="태그" modified={modified.tags} top>
    <TagPills {disabled} onremovepartial={onremovepartialtag} partial={partialTags} bind:tags />
  </PropertyRow>

  {#if showMeta}
    <PropertyRow icon={TextIcon} label="미리보기 문구" modified={modified.excerpt} top>
      <Popover
        style={css.raw(propertyTriggerStyle, { alignItems: 'flex-start', whiteSpace: 'normal' })}
        contentStyle={css.raw({ display: 'flex', flexDirection: 'column', gap: '8px', width: '380px', paddingX: '10px', paddingY: '10px' })}
        {disabled}
        offset={4}
        placement="bottom-start"
        bind:open={excerptOpen}
      >
        {#snippet trigger()}
          {#if excerpt.trim().length > 0}
            <span class={css({ lineClamp: '2', lineHeight: '[1.5]', textAlign: 'left' })}>{excerpt.trim()}</span>
          {:else}
            <span class={css({ lineClamp: '2', lineHeight: '[1.5]', textAlign: 'left', color: 'text.hint' })}>{autoExcerpt}</span>
            <span
              class={css({
                flexShrink: '0',
                marginTop: '1px',
                paddingX: '6px',
                paddingY: '1px',
                borderRadius: '4px',
                fontSize: '11px',
                fontWeight: 'semibold',
                color: 'text.muted',
                backgroundColor: 'surface.inset',
              })}
            >
              자동
            </span>
          {/if}
        {/snippet}

        <textarea
          bind:this={excerptEl}
          class={css({
            width: 'full',
            minHeight: '96px',
            padding: '10px',
            borderWidth: '1px',
            borderRadius: '6px',
            fontSize: '13px',
            lineHeight: '[1.6]',
            color: 'text.default',
            resize: 'none',
            _placeholder: { color: 'text.hint/60' },
            _focusWithin: { borderColor: 'accent.default' },
          })}
          aria-label="미리보기 문구"
          placeholder={autoExcerpt}
          bind:value={excerpt}></textarea>

        <div class={flex({ alignItems: 'center', justifyContent: 'space-between', gap: '12px' })}>
          <span class={css({ fontSize: '12px', color: 'text.hint' })}>비워 두면 본문 앞부분이 쓰여요.</span>
          <span class={css({ fontSize: '12px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>{excerpt.length}자</span>
        </div>
      </Popover>
    </PropertyRow>

    <PropertyRow icon={ImageIcon} label="썸네일" modified={modified.thumbnail}>
      {#if thumbnailId}
        <div class={css({ position: 'relative' })}>
          <button
            class={css({
              position: 'relative',
              display: 'block',
              size: '36px',
              overflow: 'hidden',
              borderRadius: '6px',
              backgroundColor: 'surface.canvas',
              _hover: { '& .thumbnail-overlay': { opacity: '100' } },
            })}
            aria-label="썸네일 변경"
            {disabled}
            onclick={handleThumbnailUpload}
            type="button"
          >
            <LoadableImg id={thumbnailId} style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })} alt="썸네일" size={128} />

            {#if !disabled}
              <span
                class={cx(
                  'thumbnail-overlay',
                  center({
                    position: 'absolute',
                    inset: '0',
                    fontSize: '11px',
                    fontWeight: 'semibold',
                    color: '[#fff]',
                    backgroundColor: '[rgba(9, 9, 12, 0.45)]',
                    opacity: '0',
                    transition: 'common',
                  }),
                )}
              >
                변경
              </span>
            {/if}
          </button>

          {#if !disabled}
            <button
              class={center({
                position: 'absolute',
                top: '-6px',
                right: '-6px',
                size: '18px',
                borderWidth: '1px',
                borderRadius: 'full',
                color: 'text.muted',
                backgroundColor: 'surface.default',
                boxShadow: 'sm',
                _hover: { color: 'danger.default' },
              })}
              aria-label="썸네일 삭제"
              onclick={() => (thumbnailId = null)}
              type="button"
              use:tooltip={{ message: '삭제', placement: 'top' }}
            >
              <Icon icon={XIcon} size={10} />
            </button>
          {/if}
        </div>
      {:else}
        <button
          class={css(propertyTriggerStyle, { color: 'text.muted', _hover: { color: 'text.default', backgroundColor: 'surface.hover' } })}
          disabled={disabled || thumbnailUploading}
          onclick={handleThumbnailUpload}
          type="button"
        >
          <Icon icon={ImageIcon} size={14} />
          <span>{thumbnailUploading ? '올리는 중' : '썸네일 올리기'}</span>
        </button>
      {/if}
    </PropertyRow>
  {/if}

  {#if metaHint}
    <p class={css(propertyNoteStyle)}>{metaHint}</p>
  {/if}

  {#if scheduleEditable}
    <PropertyRow icon={ClockIcon} label="발행 시각" modified={modified.schedule}>
      <Popover
        style={css.raw(propertyTriggerStyle)}
        contentStyle={css.raw({ display: 'flex', flexDirection: 'column', width: '300px', paddingX: '4px', paddingY: '4px' })}
        {disabled}
        offset={4}
        placement="bottom-start"
      >
        {#snippet trigger()}
          <span class={css({ fontVariantNumeric: 'tabular-nums' }, mode === 'keep' ? { color: 'text.hint' } : {})}>
            {mode === 'schedule' ? scheduleLabel : mode === 'now' ? '지금' : '여러 값'}
          </span>
          <Icon style={propertyChevronStyle} icon={ChevronDownIcon} size={14} />
        {/snippet}

        <div class={flex({ flexDirection: 'column', gap: '2px' })} aria-label="발행 시각" role="radiogroup">
          {@render whenOption('now', '지금 발행', '발행을 누르면 바로 올라가요')}
          {@render whenOption('schedule', '예약 발행', '정한 시각에 올라가요')}
        </div>

        {#if mode === 'schedule'}
          <div
            class={css({
              marginTop: '4px',
              paddingTop: '8px',
              paddingX: '8px',
              paddingBottom: '6px',
              borderTopWidth: '1px',
              borderColor: 'border.hairline',
            })}
          >
            <ScheduleField bind:parts={schedule} />
          </div>
        {/if}
      </Popover>
    </PropertyRow>
  {/if}
</div>
