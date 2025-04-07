<script lang="ts">
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import { graphql } from '$graphql';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import PageList from './PageList.svelte';
  import type { Entity } from './types';

  type Props = {
    entity: Entity;
    depth: number;
    onPointerDown: (e: PointerEvent, entity: Entity) => void;
    registerNode: (node: HTMLElement | undefined, entity: Entity & { depth: number }) => void;
  };

  let { entity, depth, onPointerDown, registerNode }: Props = $props();

  const loadEntity = graphql(`
    query PageItem_Query($id: ID!) @manual {
      entity(id: $id) {
        id
        slug

        children {
          __typename
          id
          slug
          order

          node {
            ... on Folder {
              __typename
              id
              name
            }

            ... on Post {
              __typename
              id

              content {
                __typename
                id
                title
              }
            }
          }
        }
      }
    }
  `);

  let open = $state(false);
  let itemEl: HTMLElement;

  $effect(() => {
    registerNode(itemEl, { ...entity, depth });
  });

  let children: Entity[] = $state([]);
</script>

<li
  bind:this={itemEl}
  id={entity.id}
  class={cx(entity.node?.__typename === 'Folder' ? 'dnd-item-folder' : 'dnd-item-page', css({ userSelect: 'none' }))}
  onpointerdown={(e) => onPointerDown(e, entity)}
>
  {#if entity.node?.__typename === 'Folder'}
    <details
      onmouseenter={async () => {
        const result = await loadEntity.refetch({ id: entity.id });
        children = result.entity.children;
      }}
      bind:open
    >
      <summary
        class={cx(
          'dnd-item-body',
          css({
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            paddingX: '8px',
            paddingY: '6px',
            borderRadius: '6px',
            fontSize: '14px',
            cursor: 'pointer',
            listStyleType: 'none',
            fontWeight: 'medium',
            color: 'gray.700',
            _hover: {
              backgroundColor: 'gray.100',
            },
          }),
        )}
      >
        <span class={css({ display: 'flex', alignItems: 'center', width: '16px', height: '16px', color: 'gray.500' })}>
          {#if open}
            <Icon icon={ChevronUpIcon} size={14} />
          {:else}
            <Icon icon={ChevronDownIcon} size={14} />
          {/if}
        </span>
        <span class={css({ display: 'flex', alignItems: 'center', width: '16px', height: '16px', color: 'gray.500' })}>
          <Icon icon={FolderIcon} size={14} />
        </span>
        <span class={css({ fontSize: '14px', lineHeight: '[1.2]' })}>{entity.node.name}</span>
      </summary>

      {#if children.length > 0}
        <PageList depth={depth + 1} entities={children} parent={entity} />
      {/if}
    </details>
  {:else}
    <a
      class={cx(
        'dnd-item-body',
        css({
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          paddingX: '8px',
          paddingY: '6px',
          borderRadius: '6px',
          fontWeight: 'medium',
          fontSize: '14px',
          color: 'gray.600',
          textDecoration: 'none',
          _hover: {
            backgroundColor: 'gray.100',
          },
        }),
      )}
      draggable="false"
      href="/{entity.slug}"
    >
      <span class={css({ display: 'flex', alignItems: 'center', width: '16px', height: '16px', marginLeft: '22px', color: 'gray.500' })}>
        <Icon icon={FileIcon} size={14} />
      </span>
      <span class={css({ fontSize: '14px', lineHeight: '[1.2]' })}>{entity.node?.content.title ?? '(제목 없음)'}</span>
    </a>
  {/if}
</li>
