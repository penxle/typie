<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { SegmentButtons } from '@typie/ui/components';
  import qs from 'query-string';
  import { env } from '$env/dynamic/public';
  import { Img, LoadableImg } from '$lib/components';
  import PublicationCard from '$lib/usersite/PublicationCard.svelte';
  import type { Img_image$key } from '$mearie';

  type Props = {
    siteName: string;
    siteSlug: string;
    siteUrl: string;
    siteLogo$key: Img_image$key;
    folderNames: readonly string[];
    title: string;
    subtitle: string | null;
    excerpt: string;
    tags: readonly string[];
    thumbnailId: string | null;
    timestamp: number | null;
    locked: boolean;
  };

  let { siteName, siteSlug, siteUrl, siteLogo$key, folderNames, title, subtitle, excerpt, tags, thumbnailId, timestamp, locked }: Props =
    $props();

  let tab = $state<'list' | 'link'>('list');

  const domain = $derived(siteUrl.replace(/^https?:\/\//, '').replace(/\/$/, '') || `${env.PUBLIC_USERSITE_HOST}/@${siteSlug}`);
  const previewImageUrl = $derived(
    qs.stringifyUrl({
      url: `${env.PUBLIC_API_URL}/og/preview`,
      query: { title, subtitle, thumbnailId },
    }),
  );
</script>

{#snippet skeletonRow()}
  <div class={flex({ gap: '14px', paddingY: '24px', borderTopWidth: '1px', borderColor: 'border.hairline' })}>
    <div class={flex({ flexDirection: 'column', flex: '1', gap: '7px', minWidth: '0' })}>
      <div class={css({ width: '[38%]', height: '10px', borderRadius: '4px', backgroundColor: 'surface.inset' })}></div>
      <div class={css({ width: '[58%]', height: '16px', marginTop: '8px', borderRadius: '4px', backgroundColor: 'surface.inset' })}></div>
      <div class={css({ width: 'full', height: '10px', marginTop: '6px', borderRadius: '4px', backgroundColor: 'surface.inset' })}></div>
      <div class={css({ width: '[76%]', height: '10px', borderRadius: '4px', backgroundColor: 'surface.inset' })}></div>
    </div>

    <div
      class={css({
        flexShrink: '0',
        width: '128px',
        marginTop: '3px',
        aspectRatio: '[16 / 9]',
        borderRadius: '4px',
        backgroundColor: 'surface.inset',
      })}
    ></div>
  </div>
{/snippet}

{#snippet thumbnail()}
  {#if thumbnailId}
    <LoadableImg id={thumbnailId} style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })} alt={title} size={256} />
  {/if}
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
    {tab === 'list' ? '독자가 스페이스에서 보는 모습이에요.' : '메신저나 SNS에 주소를 붙여 넣으면 이렇게 보여요.'}
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
          <Img style={css.raw({ size: '18px', borderRadius: '4px' })} alt={siteName} image$key={siteLogo$key} size={64} />

          <b class={css({ fontSize: '13px', fontWeight: 'semibold' })}>{siteName}</b>
          <span class={css({ marginLeft: 'auto', fontSize: '12px', fontFamily: 'mono', color: 'text.hint' })}>{domain}</span>
        </div>

        <div class={css({ paddingX: '16px' })}>
          <div class={css({ paddingTop: '16px' })}>
            <PublicationCard
              {excerpt}
              folders={folderNames.map((name) => ({ name }))}
              hasPassword={locked}
              {subtitle}
              {tags}
              thumbnail={thumbnailId ? thumbnail : undefined}
              {timestamp}
              title={title || '(제목 없음)'}
            />
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
