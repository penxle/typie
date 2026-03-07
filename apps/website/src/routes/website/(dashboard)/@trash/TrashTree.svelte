<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { graphql } from '$mearie';
  import TrashEntity from './TrashEntity.svelte';
  import type { DashboardLayout_TrashTree_site$key } from '$mearie';

  type Props = {
    site$key: DashboardLayout_TrashTree_site$key;
  };

  let { site$key }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_TrashTree_site on Site {
        id

        deletedEntities {
          id
          deletedAt
          node {
            __typename
          }
          ...DashboardLayout_TrashTree_TrashEntity_entity

          deletedChildren {
            id
            deletedAt
            node {
              __typename
            }
            ...DashboardLayout_TrashTree_TrashEntity_entity

            deletedChildren {
              id
              deletedAt
              node {
                __typename
              }
              ...DashboardLayout_TrashTree_TrashEntity_entity

              deletedChildren {
                id
                deletedAt
                node {
                  __typename
                }
                ...DashboardLayout_TrashTree_TrashEntity_entity
              }
            }
          }
        }
      }
    `),
    () => site$key,
  );

  const formatDateLabel = (dateStr: string) => {
    const date = dayjs(dateStr);
    const today = dayjs().startOf('day');
    const yesterday = today.subtract(1, 'day');

    if (date.isSame(today, 'day')) return '오늘';
    if (date.isSame(yesterday, 'day')) return '어제';
    if (date.year() === today.year()) return date.format('M월 D일');
    return date.format('YYYY년 M월 D일');
  };

  const groupedEntities = $derived.by(() => {
    const groups: { label: string; dateKey: string; entities: (typeof site.data.deletedEntities)[number][] }[] = [];
    const index: Record<string, (typeof site.data.deletedEntities)[number][]> = {};

    for (const entity of site.data.deletedEntities) {
      const dateKey = entity.deletedAt ? dayjs(entity.deletedAt).format('YYYY-MM-DD') : 'unknown';
      const existing = index[dateKey];
      if (existing) {
        existing.push(entity);
      } else {
        const arr = [entity];
        index[dateKey] = arr;
        groups.push({ label: formatDateLabel(dateKey), dateKey, entities: arr });
      }
    }

    return groups;
  });
</script>

<div
  class={flex({
    flexDirection: 'column',
    flexGrow: '1',
    height: 'full',
    userSelect: 'none',
  })}
  role="tree"
>
  {#if site.data.deletedEntities.length > 0}
    <div
      class={flex({
        flexDirection: 'column',
        flexGrow: '1',
        overflowY: 'auto',
      })}
    >
      <div class={css({ height: '8px', flexShrink: '0' })}></div>
      {#each groupedEntities as group (group.dateKey)}
        <div class={flex({ flexDirection: 'column' })}>
          <div
            class={css({
              position: 'sticky',
              top: '0',
              zIndex: '1',
              paddingX: '24px',
              paddingY: '8px',
              fontSize: '13px',
              fontWeight: 'semibold',
              color: 'text.subtle',
              backgroundColor: 'surface.default',
            })}
          >
            {group.label}
          </div>

          <div class={flex({ flexDirection: 'column', paddingX: '12px', paddingBottom: '8px' })}>
            {#each group.entities as entity (entity.id)}
              <TrashEntity entity$key={entity} />
            {/each}
          </div>
        </div>
      {/each}
      <div class={css({ height: '8px', flexShrink: '0' })}></div>
    </div>
  {:else}
    <div class={center({ flexGrow: '1', flexDirection: 'column', gap: '8px' })}>
      <Icon style={css.raw({ color: 'text.disabled' })} icon={Trash2Icon} size={32} />
      <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>휴지통이 비어있어요</p>
      <p class={css({ fontSize: '13px', color: 'text.disabled' })}>삭제 후 30일동안 보관돼요</p>
    </div>
  {/if}
</div>
