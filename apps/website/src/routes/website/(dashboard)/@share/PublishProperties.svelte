<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem, Popover } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ClockIcon from '~icons/lucide/clock';
  import GlobeIcon from '~icons/lucide/globe';
  import ImageIcon from '~icons/lucide/image';
  import LibraryIcon from '~icons/lucide/library';
  import TagIcon from '~icons/lucide/tag';
  import TextIcon from '~icons/lucide/text';
  import XIcon from '~icons/lucide/x';
  import { Img, LoadableImg } from '$lib/components';
  import { composeScheduledAt, formatPublishTime } from '$lib/publication/publish-form';
  import { uploadBlobAsImage } from '$lib/utils';
  import { graphql } from '$mearie';
  import PropertyRow from './PropertyRow.svelte';
  import {
    coverTileStyle,
    menuItemStyle,
    menuListStyle,
    propertyChevronStyle,
    propertyTextStyle,
    propertyTriggerStyle,
  } from './publish-styles';
  import ScheduleField from './ScheduleField.svelte';
  import TagPills from './TagPills.svelte';
  import type { ModifiedPublishFields, PublishMode, ScheduleParts } from '$lib/publication/publish-form';
  import type { DashboardLayout_Share_PublishProperties_site$key } from '$mearie';

  type Props = {
    site$key: DashboardLayout_Share_PublishProperties_site$key;
    autoExcerpt: string;
    spaceId: string | null;
    collectionId: string | null;
    tags: string[];
    excerpt: string;
    thumbnailId: string | null;
    mode: PublishMode;
    schedule: ScheduleParts;
    modified: ModifiedPublishFields;
    spaceSelectable: boolean;
    scheduleEditable: boolean;
    disabled?: boolean;
  };

  let {
    site$key,
    autoExcerpt,
    spaceId = $bindable(),
    collectionId = $bindable(),
    tags = $bindable(),
    excerpt = $bindable(),
    thumbnailId = $bindable(),
    mode = $bindable(),
    schedule = $bindable(),
    modified,
    spaceSelectable,
    scheduleEditable,
    disabled = false,
  }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_Share_PublishProperties_site on Site {
        id

        logo {
          id
          ...Img_image
        }

        spaces {
          id
          name
          url

          logo {
            id
            ...Img_image
          }

          collections {
            id
            name

            cover {
              id
              ...Img_image
            }

            publications {
              id
            }
          }
        }
      }
    `),
    () => site$key,
  );

  const spaceDomain = (url: string) => url.replace(/^https?:\/\//, '').replace(/\/$/, '');

  const spaces = $derived(site.data.spaces);
  const space = $derived(spaces.find((s) => s.id === spaceId) ?? spaces[0]);
  const collections = $derived(space?.collections ?? []);
  const collection = $derived(collections.find((c) => c.id === collectionId) ?? null);

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

{#snippet spaceLogo(target: (typeof spaces)[number], size: 16 | 20)}
  <Img
    style={css.raw({ flexShrink: '0', size: `${size}px`, borderRadius: size >= 20 ? '5px' : '4px' })}
    alt={target.name}
    image$key={target.logo ?? site.data.logo}
    size={64}
  />
{/snippet}

{#snippet collectionCover(target: (typeof collections)[number], size: 16 | 20)}
  {#if target.cover}
    <Img
      style={css.raw({ flexShrink: '0', size: `${size}px`, borderRadius: size >= 20 ? '5px' : '4px' })}
      alt={target.name}
      image$key={target.cover}
      size={64}
    />
  {:else}
    <span class={css(coverTileStyle, { size: `${size}px`, borderRadius: size >= 20 ? '5px' : '4px' })}></span>
  {/if}
{/snippet}

{#snippet menuSeparator()}
  <div class={css({ height: '1px', marginY: '2px', backgroundColor: 'border.hairline' })} role="separator"></div>
{/snippet}

{#snippet menuCheck(selected: boolean)}
  <span class={css({ flexShrink: '0', width: '14px', color: 'accent.default' })}>
    {#if selected}
      <Icon icon={CheckIcon} size={14} />
    {/if}
  </span>
{/snippet}

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
  <PropertyRow icon={GlobeIcon} label="스페이스" modified={modified.space}>
    {#if space}
      {#if spaceSelectable && spaces.length > 1}
        <Menu
          style={propertyTriggerStyle}
          buttonAriaLabel="스페이스"
          {disabled}
          listStyle={css.raw(menuListStyle, { minWidth: '240px' })}
          offset={4}
          placement="bottom-start"
        >
          {#snippet button()}
            {@render spaceLogo(space, 16)}
            <span class={css(propertyTextStyle)}>{space.name}</span>
            <Icon style={propertyChevronStyle} icon={ChevronDownIcon} size={14} />
          {/snippet}

          {#each spaces as item (item.id)}
            <MenuItem
              style={menuItemStyle}
              onclick={() => {
                if (item.id === spaceId) return;
                spaceId = item.id;
                collectionId = null;
              }}
            >
              {#snippet prefix()}
                {@render spaceLogo(item, 20)}
              {/snippet}

              <span class={flex({ flexDirection: 'column', gap: '2px', flex: '1', minWidth: '0' })}>
                <span class={css(propertyTextStyle)}>{item.name}</span>
                <span class={css({ fontSize: '11px', fontWeight: 'normal', fontFamily: 'mono', color: 'text.hint' })}>
                  {spaceDomain(item.url)}
                </span>
              </span>

              {#snippet suffix()}
                {@render menuCheck(item.id === space.id)}
              {/snippet}
            </MenuItem>
          {/each}
        </Menu>
      {:else}
        <span class={css(propertyTriggerStyle, { cursor: 'default', _hover: { backgroundColor: 'transparent' } })}>
          {@render spaceLogo(space, 16)}
          <span class={css(propertyTextStyle)}>{space.name}</span>
        </span>
      {/if}
    {/if}
  </PropertyRow>

  <PropertyRow icon={LibraryIcon} label="시리즈" modified={modified.collection}>
    <Menu
      style={propertyTriggerStyle}
      buttonAriaLabel="시리즈"
      {disabled}
      listStyle={css.raw(menuListStyle, { minWidth: '260px' })}
      offset={4}
      placement="bottom-start"
    >
      {#snippet button()}
        {#if collection}
          {@render collectionCover(collection, 16)}
          <span class={css(propertyTextStyle)}>{collection.name}</span>
        {:else}
          <span class={css(propertyTextStyle, { color: 'text.hint' })}>없음</span>
        {/if}
        <Icon style={propertyChevronStyle} icon={ChevronDownIcon} size={14} />
      {/snippet}

      <MenuItem style={menuItemStyle} onclick={() => (collectionId = null)}>
        없음

        {#snippet suffix()}
          {@render menuCheck(collectionId === null)}
        {/snippet}
      </MenuItem>

      {#if collections.length > 0}
        {@render menuSeparator()}
      {/if}

      {#each collections as item (item.id)}
        <MenuItem style={menuItemStyle} onclick={() => (collectionId = item.id)}>
          {#snippet prefix()}
            {@render collectionCover(item, 20)}
          {/snippet}

          <span class={css(propertyTextStyle, { flex: '1' })}>{item.name}</span>

          {#snippet suffix()}
            <span class={css({ fontSize: '12px', fontWeight: 'normal', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>
              글 {item.publications.length}개
            </span>

            {@render menuCheck(collectionId === item.id)}
          {/snippet}
        </MenuItem>
      {/each}

      {@render menuSeparator()}

      <p
        class={css({ paddingX: '8px', paddingTop: '6px', paddingBottom: '4px', fontSize: '12px', lineHeight: '[1.5]', color: 'text.hint' })}
      >
        새 시리즈는 스페이스 설정에서 만들 수 있어요.
      </p>
    </Menu>
  </PropertyRow>

  <PropertyRow icon={TagIcon} label="태그" modified={modified.tags} top>
    <TagPills {disabled} bind:tags />
  </PropertyRow>

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
          <span class={css({ fontVariantNumeric: 'tabular-nums' })}>{mode === 'schedule' ? scheduleLabel : '지금'}</span>
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
