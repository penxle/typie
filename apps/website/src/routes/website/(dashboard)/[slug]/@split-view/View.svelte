<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon, RingSpinner } from '@typie/ui/components';
  import { clamp } from '@typie/ui/utils';
  import { EntityState } from '@/enums';
  import FileXIcon from '~icons/lucide/file-x';
  import XIcon from '~icons/lucide/x';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { fragment, graphql } from '$graphql';
  import Canvas from '../@canvas/Canvas.svelte';
  import Editor from '../Editor.svelte';
  import CloseSplitView from './CloseSplitView.svelte';
  import { getSplitViewContext, setupViewContext } from './context.svelte';
  import { VIEW_MIN_SIZE } from './utils';
  import ViewDropZone from './ViewDropZone.svelte';
  import type { SplitViews_View_query } from '$graphql';
  import type { SplitViewItem } from './context.svelte';

  type Props = {
    viewItem: SplitViewItem;
    $query: SplitViews_View_query;
  };

  let { viewItem, $query: _query }: Props = $props();

  const query = fragment(
    _query,
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
          }
        }

        ...Canvas_query
        ...Editor_query
      }
    `),
  );

  const splitView = getSplitViewContext();

  const focused = $derived(viewItem.id === splitView.state.current.focusedViewId);
  const entity = $derived.by(() => $query.entities.find((entity) => entity.slug === viewItem.slug));

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
    borderWidth: '1px',
    boxShadow: '[0 3px 6px -2px {colors.shadow.default/3}, 0 1px 1px {colors.shadow.default/5}]',
    borderRadius: '4px',
    overflow: 'hidden',
    borderColor: focused && splitView.state.current.enabled ? 'border.strong' : 'transparent',
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
    {#if entity.state === EntityState.ACTIVE}
      {#key entity.slug}
        {#if entity.node.__typename === 'Post'}
          <Editor {$query} {focused} slug={entity.slug} />
        {:else if entity.node.__typename === 'Canvas'}
          <Canvas {$query} {focused} slug={entity.slug} />
        {/if}
      {/key}
    {:else}
      {@const name = entity.node.__typename === 'Post' ? '포스트' : '캔버스'}
      {#if focused}
        <Helmet title={`삭제된 ${name}`} />
      {/if}

      {#if splitView.state.current.enabled}
        <CloseSplitView style={css.raw({ position: 'absolute', top: '6px', right: '8px' })}>
          <Icon icon={XIcon} size={16} />
        </CloseSplitView>
      {/if}

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
  {:else}
    <div class={center({ size: 'full' })}>
      {#if splitView.state.current.enabled}
        <CloseSplitView style={css.raw({ position: 'absolute', top: '6px', right: '8px' })}>
          <Icon icon={XIcon} size={16} />
        </CloseSplitView>
      {/if}

      <RingSpinner style={css.raw({ size: '24px', color: 'text.subtle' })} />
    </div>
  {/if}
</div>
