<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, SegmentButtons } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import qs from 'query-string';
  import LockIcon from '~icons/lucide/lock';
  import { env } from '$env/dynamic/public';
  import { Img, LoadableImg } from '$lib/components';
  import type { Img_image$key } from '$mearie';

  type Props = {
    spaceName: string;
    spaceSlug: string;
    spaceUrl: string;
    spaceLogo$key: Img_image$key;
    title: string;
    subtitle: string | null;
    excerpt: string;
    tags: readonly string[];
    collectionName: string | null;
    thumbnailId: string | null;
    date: Date;
    locked: boolean;
  };

  let { spaceName, spaceSlug, spaceUrl, spaceLogo$key, title, subtitle, excerpt, tags, collectionName, thumbnailId, date, locked }: Props =
    $props();

  let tab = $state<'list' | 'link'>('list');

  const domain = $derived(spaceUrl.replace(/^https?:\/\//, '').replace(/\/$/, '') || `${spaceSlug}.typie.me`);
  const previewImageUrl = $derived(
    qs.stringifyUrl({
      url: `${env.PUBLIC_API_URL}/og/preview`,
      query: { title, subtitle, thumbnailId },
    }),
  );
  const formattedDate = $derived(dayjs(date).format('YYYY. M. D.'));
</script>

{#snippet skeletonRow()}
  <div class={flex({ gap: '14px', paddingY: '16px', borderBottomWidth: '1px', borderColor: 'border.hairline' })}>
    <div class={flex({ flexDirection: 'column', flex: '1', gap: '7px', minWidth: '0' })}>
      <div class={css({ width: '[58%]', height: '14px', borderRadius: '4px', backgroundColor: 'surface.inset' })}></div>
      <div class={css({ width: 'full', height: '10px', marginTop: '5px', borderRadius: '4px', backgroundColor: 'surface.inset' })}></div>
      <div class={css({ width: '[76%]', height: '10px', borderRadius: '4px', backgroundColor: 'surface.inset' })}></div>
      <div class={css({ width: '[38%]', height: '8px', marginTop: '7px', borderRadius: '4px', backgroundColor: 'surface.inset' })}></div>
    </div>

    <div class={css({ flexShrink: '0', size: '72px', borderRadius: '6px', backgroundColor: 'surface.inset' })}></div>
  </div>
{/snippet}

<div class={flex({ flexDirection: 'column', gap: '12px', minHeight: '0' })}>
  <SegmentButtons
    style={css.raw({ flex: 'none' })}
    items={[
      { label: '스페이스 목록', value: 'list' as const },
      { label: '링크 미리보기', value: 'link' as const },
    ]}
    onselect={(value) => (tab = value)}
    size="sm"
    value={tab}
  />

  <p class={css({ flex: 'none', fontSize: '12px', color: 'text.hint' })}>
    {tab === 'list' ? '독자가 스페이스 홈에서 보는 모습이에요.' : '메신저나 SNS에 주소를 붙여 넣으면 이렇게 보여요.'}
  </p>

  <div class={css({ flex: '1', minHeight: '0', marginX: '-24px', paddingX: '24px', paddingBottom: '24px', overflowY: 'auto' })}>
    {#if tab === 'list'}
      <div
        class={css({
          borderWidth: '1px',
          borderColor: 'border.hairline',
          borderRadius: '10px',
          backgroundColor: 'surface.default',
          overflow: 'hidden',
        })}
      >
        <div
          class={flex({
            alignItems: 'center',
            gap: '8px',
            paddingX: '16px',
            paddingY: '12px',
            borderBottomWidth: '1px',
            borderColor: 'border.hairline',
          })}
        >
          <Img style={css.raw({ size: '18px', borderRadius: '4px' })} alt={spaceName} image$key={spaceLogo$key} size={64} />

          <b class={css({ fontSize: '13px', fontWeight: 'semibold' })}>{spaceName}</b>
          <span class={css({ marginLeft: 'auto', fontSize: '12px', fontFamily: 'mono', color: 'text.hint' })}>{domain}</span>
        </div>

        <div class={css({ paddingX: '16px' })}>
          <div class={flex({ gap: '14px', paddingY: '16px', borderBottomWidth: '1px', borderColor: 'border.hairline' })}>
            <div class={flex({ flexDirection: 'column', flex: '1', gap: '4px', minWidth: '0' })}>
              <h4 class={css({ fontSize: '16px', fontWeight: 'semibold', lineClamp: '2' })}>
                {#if locked}
                  <span class={css({ display: 'inline-flex', marginRight: '6px', verticalAlign: '[-1px]', color: 'text.muted' })}>
                    <Icon icon={LockIcon} size={14} />
                  </span>
                {/if}{title || '(제목 없음)'}
              </h4>

              {#if subtitle}
                <h5 class={css({ fontSize: '13px', color: 'text.muted', lineClamp: '1' })}>{subtitle}</h5>
              {/if}

              <p class={css({ marginTop: '4px', fontSize: '13px', lineHeight: '[1.6]', color: 'text.muted', lineClamp: '2' })}>{excerpt}</p>

              <div
                class={flex({ alignItems: 'center', flexWrap: 'wrap', gap: '6px', marginTop: '6px', fontSize: '12px', color: 'text.hint' })}
              >
                <span class={css({ fontVariantNumeric: 'tabular-nums' })}>{formattedDate}</span>

                {#if collectionName}
                  <span>·</span>
                  <span>{collectionName}</span>
                {/if}

                {#if tags.length > 0}
                  <span>·</span>
                  <span class={flex({ gap: '6px' })}>
                    {#each tags as tag (tag)}
                      <span>#{tag}</span>
                    {/each}
                  </span>
                {/if}
              </div>
            </div>

            {#if thumbnailId}
              <div
                class={css({ flexShrink: '0', size: '72px', borderRadius: '6px', overflow: 'hidden', backgroundColor: 'surface.canvas' })}
              >
                <LoadableImg
                  id={thumbnailId}
                  style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
                  alt="썸네일"
                  size={128}
                />
              </div>
            {/if}
          </div>

          {@render skeletonRow()}
          {@render skeletonRow()}
        </div>
      </div>
    {:else}
      <div
        class={css({
          overflow: 'hidden',
          borderWidth: '1px',
          borderColor: 'border.hairline',
          borderRadius: '10px',
          backgroundColor: 'surface.default',
          boxShadow: 'md',
        })}
      >
        <img
          class={css({
            display: 'block',
            width: 'full',
            aspectRatio: '[1200 / 630]',
            objectFit: 'cover',
            backgroundColor: 'surface.canvas',
          })}
          alt="링크 미리보기 이미지"
          src={previewImageUrl}
        />

        <div class={css({ paddingX: '12px', paddingTop: '10px', paddingBottom: '12px' })}>
          <b
            class={css({
              display: 'block',
              overflow: 'hidden',
              fontSize: '13px',
              fontWeight: 'semibold',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            })}
          >
            {title || '(제목 없음)'}
          </b>
          <p class={css({ marginTop: '2px', fontSize: '12px', lineHeight: '[1.5]', color: 'text.muted', lineClamp: '2' })}>{excerpt}</p>
          <span class={css({ display: 'block', marginTop: '6px', fontSize: '11px', color: 'text.hint' })}>{domain}</span>
        </div>
      </div>
    {/if}
  </div>
</div>
