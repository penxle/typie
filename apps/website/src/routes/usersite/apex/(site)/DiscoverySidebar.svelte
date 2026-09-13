<script lang="ts">
  import { titlePageColors } from '@typie/lib/title-page';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, TimeAgo } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import { env } from '$env/dynamic/public';
  import { Img } from '$lib/components';
  import TagChip from '../@[slug]/TagChip.svelte';
  import DiscoveryFooter from './DiscoveryFooter.svelte';
  import { discoveryTagPath } from './paths';
  import { twoWaySticky } from './two-way-sticky';
  import type { DataOf } from '@mearie/svelte';
  import type { UsersiteApexFeedLayout_Query } from '$mearie';

  type Props = {
    discovery: DataOf<UsersiteApexFeedLayout_Query>['discovery'];
    loggedIn: boolean;
    currentTag: string | null;
    headerBottom: number;
    viewportHeight: number;
  };

  let { discovery, loggedIn, currentTag, headerBottom, viewportHeight }: Props = $props();

  const heading = css.raw({ marginBottom: '12px', fontSize: '13px', fontWeight: 'semibold' });

  const rows = css.raw({ display: 'flex', flexDirection: 'column', marginY: '-8px' });

  const row = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '12px',
    minWidth: '0',
    paddingY: '8px',
    _hover: { '& [data-row-name]': { color: 'text.muted' } },
  });

  const rowName = css.raw({ fontSize: '14px', fontWeight: 'semibold', lineHeight: '[1.4]', transition: 'colors', truncate: true });
  const rowText = css.raw({ fontSize: '12px', lineHeight: '[1.45]', color: 'text.muted', truncate: true });
  const rowMeta = css.raw({ fontSize: '12px', lineHeight: '[1.45]', color: 'text.hint', truncate: true });
</script>

<aside
  class={flex({
    flexDirection: 'column',
    gap: '36px',
    minWidth: '0',
    lg: { position: 'sticky', top: '[calc(var(--usersite-sticky-header-bottom, 52px) + 28px)]' },
    lgDown: { marginTop: '16px', paddingTop: '40px', borderTopWidth: '1px', borderColor: 'border.hairline' },
  })}
  use:twoWaySticky={{ headerBottom, viewportHeight }}
>
  {#if !loggedIn}
    <section
      class={flex({ flexDirection: 'column', gap: '16px', padding: '20px', borderRadius: '12px', backgroundColor: 'surface.inset' })}
    >
      <div>
        <h2 class={css({ fontSize: '16px', fontWeight: 'bold', lineHeight: '[1.4]', letterSpacing: '-0.015em', textWrap: 'balance' })}>
          읽다가 쓰고 싶어지면, 타이피에서 시작하세요
        </h2>
        <p class={css({ marginTop: '4px', fontSize: '14px', lineHeight: '[1.6]', color: 'text.muted' })}>
          타이피로 어디서든 즐겁게 글을 이어나가세요. 작성, 정리, 공유까지 글쓰기의 모든 과정을 함께 해요.
        </p>
      </div>
      <Button style={css.raw({ gap: '4px' })} external href={env.PUBLIC_WEBSITE_URL} size="md" type="link" variant="primary">
        타이피 알아보기
        <Icon icon={ChevronRightIcon} size={14} />
      </Button>
    </section>
  {/if}

  {#if discovery.recentSpaces.length > 0}
    <div>
      <h2 class={css(heading)}>새 글을 올린 스페이스</h2>
      <div class={css(rows)}>
        {#each discovery.recentSpaces as item (item.space.id)}
          <a class={css(row)} href={item.space.url}>
            <Img
              style={css.raw({
                flexShrink: '0',
                size: '36px',
                borderRadius: '9px',
                objectFit: 'cover',
                boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.06)]',
              })}
              alt={`${item.space.name} 로고`}
              image$key={item.space.logo}
              size={96}
            />
            <div class={css({ flex: '1', minWidth: '0' })}>
              <div class={css(rowName)} data-row-name>{item.space.name}</div>
              <div class={css(rowText)}>{item.publication.title}</div>
            </div>
            <TimeAgo style={css.raw(rowMeta, { flexShrink: '0' })} timestamp={dayjs(item.publication.publishedAt).valueOf()} />
          </a>
        {/each}
      </div>
    </div>
  {/if}

  {#if discovery.recentCollections.length > 0}
    <div>
      <h2 class={css(heading)}>새 회차가 올라온 시리즈</h2>
      <div class={css(rows)}>
        {#each discovery.recentCollections as item (item.collection.id)}
          <a class={css(row)} href={`${item.space.url}/s/${item.collection.permalink}`}>
            <div
              style:background-color={item.collection.cover ? undefined : titlePageColors(item.collection.name).base}
              class={css({
                position: 'relative',
                flexShrink: '0',
                width: '36px',
                height: '54px',
                borderRadius: '6px',
                backgroundColor: 'surface.inset',
                boxShadow: '[inset 0 0 0 1px token(colors.border.hairline)]',
                overflow: 'hidden',
              })}
            >
              {#if item.collection.cover}
                <Img
                  style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
                  alt={item.collection.name}
                  image$key={item.collection.cover}
                  size={96}
                />
              {/if}
            </div>
            <div class={css({ flex: '1', minWidth: '0' })}>
              <div class={css(rowName)} data-row-name>{item.collection.name}</div>
              <div class={css(rowMeta)}>{item.space.name} · 글 {item.collection.publicationCount}개</div>
            </div>
          </a>
        {/each}
      </div>
    </div>
  {/if}

  {#if discovery.tags.length > 0}
    <div>
      <h2 class={css(heading)}>태그</h2>
      <div class={flex({ flexWrap: 'wrap', gap: '6px' })}>
        {#each discovery.tags as tag (tag.name)}
          <TagChip name={tag.name} count={tag.count} current={tag.name === currentTag} href={discoveryTagPath(tag.name)} noscroll={false} />
        {/each}
      </div>
    </div>
  {/if}

  <DiscoveryFooter />
</aside>
