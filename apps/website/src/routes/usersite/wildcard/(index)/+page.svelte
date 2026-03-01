<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import FolderIcon from '~icons/lucide/folder';
  import { Img } from '$lib/components';
  import { hydrateQuery } from '$lib/graphql';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const folders = $derived(query.data.siteView.entities.filter((entity) => entity.node.__typename === 'FolderView'));
  const documents = $derived(query.data.siteView.entities.filter((entity) => entity.node.__typename === 'DocumentView'));
</script>

<svelte:head>
  <meta name="robots" content="noindex, nofollow" />
</svelte:head>

<Helmet
  description={`${query.data.siteView.name}에서 공유된 폴더 ${folders.length}개, 문서 ${documents.length}개를 확인하세요.`}
  title={query.data.siteView.name}
/>

<div class={flex({ flexDirection: 'column', alignItems: 'center', width: 'full', minHeight: 'full' })}>
  <div
    class={flex({
      flexDirection: 'column',
      flexGrow: '1',
      paddingX: { base: '20px', md: '40px' },
      paddingTop: { base: '48px', md: '80px' },
      paddingBottom: '120px',
      width: 'full',
      maxWidth: '680px',
    })}
  >
    <header class={css({ marginBottom: { base: '56px', md: '72px' } })}>
      {#if query.data.siteView.logo}
        <Img
          style={css.raw({ size: '44px', borderRadius: '10px', objectFit: 'cover', marginBottom: '20px' })}
          alt={`${query.data.siteView.name} 로고`}
          image$key={query.data.siteView.logo}
          size={48}
        />
      {/if}

      <h1 class={css({ fontSize: '22px', fontWeight: 'bold', letterSpacing: '-0.01em', lineHeight: '[1.3]' })}>
        {query.data.siteView.name}
      </h1>

      {#if folders.length > 0 || documents.length > 0}
        <p class={css({ marginTop: '8px', fontSize: '14px', color: 'text.muted' })}>
          {#if folders.length > 0}
            폴더 {folders.length}개
          {/if}
          {#if folders.length > 0 && documents.length > 0}
            ·
          {/if}
          {#if documents.length > 0}
            문서 {documents.length}개
          {/if}
        </p>
      {/if}
    </header>

    {#if folders.length > 0}
      <section class={css({ marginBottom: '48px' })}>
        <div class={flex({ flexDirection: 'column', gap: '4px' })}>
          {#each folders as entity (entity.id)}
            {#if entity.node.__typename === 'FolderView'}
              <a
                class={flex({
                  align: 'center',
                  gap: '14px',
                  paddingY: '12px',
                  cursor: 'pointer',
                  _hover: { '& .folder-name': { color: 'text.muted' } },
                })}
                href={`/${entity.slug}`}
              >
                {#if entity.node.thumbnail}
                  <div
                    class={css({
                      flexShrink: '0',
                      size: '48px',
                      borderRadius: '8px',
                      backgroundColor: 'surface.subtle',
                      overflow: 'hidden',
                    })}
                  >
                    <Img
                      style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
                      alt={entity.node.name}
                      image$key={entity.node.thumbnail}
                      size={48}
                    />
                  </div>
                {:else}
                  <div
                    class={flex({
                      alignItems: 'center',
                      justifyContent: 'center',
                      flexShrink: '0',
                      size: '48px',
                      borderRadius: '8px',
                      backgroundColor: 'surface.subtle',
                    })}
                  >
                    <Icon style={css.raw({ color: 'text.faint' })} icon={FolderIcon} size={18} />
                  </div>
                {/if}

                <div class={css({ flex: '1', minWidth: '0' })}>
                  <p
                    class={css({
                      fontSize: '15px',
                      fontWeight: 'medium',
                      color: 'text.default',
                      truncate: true,
                      transition: 'colors',
                    })}
                  >
                    <span class="folder-name">{entity.node.name}</span>
                  </p>
                  {#if entity.node.folderCount > 0 || entity.node.documentCount > 0}
                    <p class={css({ marginTop: '2px', fontSize: '13px', color: 'text.faint' })}>
                      {#if entity.node.folderCount > 0}
                        폴더 {entity.node.folderCount}개
                      {/if}
                      {#if entity.node.folderCount > 0 && entity.node.documentCount > 0}
                        ·
                      {/if}
                      {#if entity.node.documentCount > 0}
                        문서 {entity.node.documentCount}개
                      {/if}
                    </p>
                  {/if}
                </div>

                <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={ChevronRightIcon} size={16} />
              </a>
            {/if}
          {/each}
        </div>
      </section>
    {/if}

    {#if documents.length > 0}
      <section>
        <div class={flex({ flexDirection: 'column' })}>
          {#each documents as entity, index (entity.id)}
            {#if entity.node.__typename === 'DocumentView'}
              <a
                class={flex({
                  gap: '24px',
                  paddingY: '20px',
                  borderTopWidth: index === 0 ? '0' : '1px',
                  borderColor: 'border.subtle',
                  cursor: 'pointer',
                  _hover: { '& .document-title': { color: 'text.muted' } },
                })}
                href={`/${entity.slug}`}
              >
                <div class={css({ flex: '1', minWidth: '0' })}>
                  <h2
                    class={css({
                      fontSize: '16px',
                      fontWeight: 'semibold',
                      lineHeight: '[1.5]',
                      letterSpacing: '-0.01em',
                      lineClamp: '2',
                      transition: 'colors',
                    })}
                  >
                    <span class="document-title">{entity.node.title}</span>
                  </h2>

                  {#if entity.node.subtitle}
                    <h3
                      class={css({
                        marginTop: '2px',
                        fontSize: '14px',
                        fontWeight: 'medium',
                        lineHeight: '[1.5]',
                        color: 'text.subtle',
                        lineClamp: '2',
                      })}
                    >
                      {entity.node.subtitle}
                    </h3>
                  {/if}

                  {#if entity.node.excerpt}
                    <p
                      class={css({
                        marginTop: '6px',
                        fontSize: '14px',
                        lineHeight: '[1.6]',
                        color: 'text.muted',
                        lineClamp: '2',
                      })}
                    >
                      {entity.node.excerpt}
                    </p>
                  {/if}

                  {#if query.data.siteView.dateDisplay !== 'NONE'}
                    <p class={css({ marginTop: '10px', fontSize: '13px', color: 'text.faint' })}>
                      {dayjs(query.data.siteView.dateDisplay === 'CREATED_AT' ? entity.node.createdAt : entity.node.updatedAt).format(
                        'YYYY. M. D.',
                      )}
                    </p>
                  {/if}
                </div>

                {#if entity.node.thumbnail}
                  <div
                    class={css({
                      flexShrink: '0',
                      width: { base: '72px', md: '100px' },
                      aspectRatio: '1/1',
                      borderRadius: '6px',
                      backgroundColor: 'surface.subtle',
                      overflow: 'hidden',
                    })}
                  >
                    <Img
                      style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
                      alt={entity.node.title}
                      image$key={entity.node.thumbnail}
                      size={256}
                    />
                  </div>
                {/if}
              </a>
            {/if}
          {/each}
        </div>
      </section>
    {/if}

    {#if folders.length === 0 && documents.length === 0}
      <div
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          paddingY: '80px',
        })}
      >
        <p class={css({ fontSize: '14px', color: 'text.faint' })}>공유된 콘텐츠가 없어요</p>
      </div>
    {/if}
  </div>
</div>
