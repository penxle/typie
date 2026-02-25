<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import { clamp } from '@typie/ui/utils';
  import { EntityState } from '@/enums';
  import FileXIcon from '~icons/lucide/file-x';
  import XIcon from '~icons/lucide/x';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { graphql } from '$mearie';
  import Document from '../Document.svelte';
  import Editor from '../Editor.svelte';
  import CloseSplitView from './CloseSplitView.svelte';
  import { getSplitViewContext, setupViewContext } from './context.svelte';
  import { replaceSplitView, VIEW_MIN_SIZE } from './utils';
  import ViewDropZone from './ViewDropZone.svelte';
  import type { SplitViews_View_query$key } from '$mearie';
  import type { SplitViewItem } from './context.svelte';

  type Props = {
    viewItem: SplitViewItem;
    query$key: SplitViews_View_query$key;
  };

  let { viewItem, query$key }: Props = $props();

  const query = createFragment(
    graphql(`
      fragment SplitViews_View_query on Query {
        me @required {
          id
        }

        entities(slugs: $slugs) {
          id
          slug
          state

          node {
            __typename

            ... on Post {
              id

              document {
                id

                entity {
                  id
                  slug
                }
              }
            }
          }
        }

        ...Document_query
        ...Editor_query
      }
    `),
    () => query$key,
  );

  const splitView = getSplitViewContext();

  const focused = $derived(viewItem.id === splitView.state.current.focusedViewId);
  const entity = $derived.by(() => query.data.entities.find((entity) => entity.slug === viewItem.slug));

  $effect(() => {
    if (entity?.node.__typename === 'Post' && entity.node.document) {
      const documentSlug = entity.node.document.entity.slug;

      if (splitView.state.current.view) {
        splitView.state.current.view = replaceSplitView(splitView.state.current.view, viewItem.id, documentSlug);
      }

      if (focused) {
        goto(`/${documentSlug}`, { replaceState: true });
      }
    }
  });

  const sizePercentage = $derived.by(() => {
    const v = splitView.state.current.currentPercentages?.[viewItem.id];
    return v == null || Number.isNaN(v) ? 100 : clamp(v, 0, 100);
  });

  let viewElement = $state<HTMLElement>();

  const handleFocus = (viewItem: SplitViewItem) => {
    splitView.state.current.focusedViewId = viewItem.id;
    if (page.params.slug !== viewItem.slug) {
      goto(`/${viewItem.slug}`, { keepFocus: true });
    }
  };

  setupViewContext(viewItem);
</script>

<div
  bind:this={viewElement}
  style:flex-basis={`${sizePercentage}%`}
  style:min-width={`${VIEW_MIN_SIZE}px`}
  style:min-height={`${VIEW_MIN_SIZE}px`}
  class={flex({
    position: 'relative',
    flex: '1',
    size: 'full',
    backgroundColor: 'surface.default',
    overflow: 'hidden',
  })}
  data-view-id={viewItem.id}
  onclick={() => {
    handleFocus(viewItem);
  }}
  onfocusin={() => {
    handleFocus(viewItem);
  }}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      handleFocus(viewItem);
    }
  }}
  role="tabpanel"
  tabindex={0}
>
  <ViewDropZone {viewElement} {viewItem} />
  {#if entity}
    {#key entity.id}
      {#if entity?.state === EntityState.ACTIVE}
        {#if entity?.node.__typename === 'Post'}
          <Editor {focused} query$key={query.data} slug={entity.slug} />
        {:else if entity?.node.__typename === 'Document'}
          <Document {focused} query$key={query.data} slug={entity.slug} />
        {/if}
      {:else}
        {@const name = entity?.node.__typename === 'Post' ? '포스트' : '문서'}
        {#if focused}
          <Helmet title={`삭제된 ${name}`} />
        {/if}

        <CloseSplitView style={css.raw({ position: 'absolute', top: '6px', right: '8px' })}>
          <Icon icon={XIcon} size={16} />
        </CloseSplitView>

        <div class={center({ flexDirection: 'column', gap: '20px', size: 'full', textAlign: 'center' })}>
          <Icon style={css.raw({ size: '56px', color: 'text.subtle', '& *': { strokeWidth: '[1.25px]' } })} icon={FileXIcon} />

          <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
            <h1 class={css({ fontSize: '16px', fontWeight: 'bold', color: 'text.subtle' })}>{name}가 삭제되었어요</h1>
            <p class={css({ fontSize: '14px', color: 'text.faint' })}>
              {name}가 삭제되어 더 이상 접근할 수 없어요.
              <br />
              다른 {name}를 선택해주세요
            </p>
          </div>
        </div>
      {/if}
    {/key}
  {:else}
    <div class={center({ size: 'full' })}>
      <CloseSplitView style={css.raw({ position: 'absolute', top: '6px', right: '8px' })}>
        <Icon icon={XIcon} size={16} />
      </CloseSplitView>

      <Logo
        class={css({
          size: '32px',
          filter: '[grayscale(100%)]',
          animation: 'pulse 2s ease-in-out infinite',
        })}
      />
    </div>
  {/if}
</div>
