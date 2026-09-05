<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import ClipboardCopyIcon from '~icons/lucide/clipboard-copy';
  import ClipboardPasteIcon from '~icons/lucide/clipboard-paste';
  import MinusIcon from '~icons/lucide/minus';
  import ScissorsIcon from '~icons/lucide/scissors';
  import TrashIcon from '~icons/lucide/trash';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import { createEntityTreeRevealRequest, entityTreeRevealState } from '../@tree/entity-reveal.svelte';
  import { getNextSiblingOrder } from '../@tree/utils';
  import IdCopyMenuItem from '../IdCopyMenuItem.svelte';
  import { showPasteToast } from './paste-toast';

  type Props = {
    divider: {
      id: string;
      createdAt: string;
    };
    entity: {
      id: string;
      order: string;
      parent?: { id: string } | null;
      site: { id: string };
    };
    via: 'tree';
  };

  let { divider, entity, via }: Props = $props();

  const app = getAppContext();

  const [createDivider] = createMutation(
    graphql(`
      mutation DividerMenu_CreateDivider_Mutation($input: CreateDividerInput!) {
        createDivider(input: $input) {
          id

          entity {
            id

            container {
              ... on Site {
                id

                entities {
                  id

                  node {
                    __typename
                  }

                  ...DashboardLayout_EntityTree_Entity_entity
                }
              }

              ... on Entity {
                id

                children {
                  id

                  node {
                    __typename
                  }

                  ...DashboardLayout_EntityTree_Entity_entity
                }
              }
            }
          }
        }
      }
    `),
  );

  const [deleteEntities] = createMutation(
    graphql(`
      mutation DividerMenu_DeleteEntities_Mutation($input: DeleteEntitiesInput!) {
        deleteEntities(input: $input) {
          id

          site {
            id
            ...DashboardLayout_EntityTree_site
            ...DashboardLayout_TrashModal_site
          }

          container {
            ... on Site {
              id

              entities {
                id

                node {
                  __typename
                }

                ...DashboardLayout_EntityTree_Entity_entity
              }
            }

            ... on Entity {
              id

              children {
                id

                node {
                  __typename
                }

                ...DashboardLayout_EntityTree_Entity_entity
              }
            }
          }
        }
      }
    `),
  );

  const [moveEntities] = createMutation(
    graphql(`
      mutation DividerMenu_MoveEntities_Mutation($input: MoveEntitiesInput!) {
        moveEntities(input: $input) {
          id

          site {
            id
            ...DashboardLayout_EntityTree_site
          }

          container {
            ... on Site {
              id

              entities {
                id

                node {
                  __typename
                }

                ...DashboardLayout_EntityTree_Entity_entity
              }
            }

            ... on Entity {
              id

              children {
                id

                node {
                  __typename
                }

                ...DashboardLayout_EntityTree_Entity_entity
              }
            }
          }

          children {
            id

            node {
              __typename
            }

            ...DashboardLayout_EntityTree_Entity_entity
          }

          ancestors {
            id

            node {
              __typename

              ... on Folder {
                id
                name
              }
            }
          }

          parent {
            id
          }
        }
      }
    `),
  );

  const [copyEntities] = createMutation(
    graphql(`
      mutation DividerMenu_CopyEntities_Mutation($input: CopyEntitiesInput!) {
        copyEntities(input: $input) {
          id

          site {
            id
            ...DashboardLayout_EntityTree_site
          }

          container {
            ... on Site {
              id

              entities {
                id

                node {
                  __typename
                }

                ...DashboardLayout_EntityTree_Entity_entity
              }
            }

            ... on Entity {
              id

              children {
                id

                node {
                  __typename
                }

                ...DashboardLayout_EntityTree_Entity_entity
              }
            }
          }
        }
      }
    `),
  );
</script>

<MenuItem
  icon={ClipboardCopyIcon}
  onclick={() => {
    app.state.clipboard = {
      mode: 'copy',
      entityIds: [entity.id],
      sourceSiteId: entity.site.id,
    };
  }}
>
  복사
</MenuItem>

<MenuItem
  icon={ScissorsIcon}
  onclick={() => {
    app.state.clipboard = {
      mode: 'cut',
      entityIds: [entity.id],
      sourceSiteId: entity.site.id,
    };
  }}
>
  잘라내기
</MenuItem>

{#if app.state.clipboard}
  <MenuItem
    icon={ClipboardPasteIcon}
    onclick={() => {
      const clipboard = app.state.clipboard;
      if (!clipboard) return;

      if (!SubscribeModal.gate('entity_paste')) {
        return;
      }

      const upperOrder = getNextSiblingOrder(entity.id) ?? null;
      const count = clipboard.entityIds.length;

      const promise = (async () => {
        if (clipboard.mode === 'cut') {
          const isCrossSite = clipboard.sourceSiteId !== entity.site.id;
          await moveEntities({
            input: {
              entityIds: clipboard.entityIds,
              parentEntityId: entity.parent?.id ?? null,
              lowerOrder: entity.order,
              upperOrder,
              ...(isCrossSite && { targetSiteId: entity.site.id }),
            },
          });
          if (isCrossSite) {
            cache.invalidate({ __typename: 'Site', id: clipboard.sourceSiteId, $field: 'entities' });
          }
          app.state.clipboard = undefined;
        } else {
          await copyEntities({
            input: {
              entityIds: clipboard.entityIds,
              targetSiteId: entity.site.id,
              parentEntityId: entity.parent?.id ?? null,
              lowerOrder: entity.order,
              upperOrder,
            },
          });
        }
      })();

      showPasteToast(promise, count);
    }}
  >
    아래에 붙여넣기
  </MenuItem>
{/if}

<MenuItem
  icon={MinusIcon}
  onclick={async () => {
    if (!SubscribeModal.gate('divider_menu_create_divider')) {
      return;
    }

    const resp = await createDivider({
      input: {
        siteId: entity.site.id,
        parentEntityId: entity.parent?.id ?? null,
        lowerOrder: entity.order,
        upperOrder: getNextSiblingOrder(entity.id) ?? null,
      },
    });

    mixpanel.track('create_divider', { via });
    entityTreeRevealState.set(createEntityTreeRevealRequest(resp.createDivider.entity.id, entity.parent ? [entity.parent.id] : [], false));
  }}
>
  아래에 구분선 삽입
</MenuItem>

<HorizontalDivider color="secondary" />

<MenuItem
  icon={TrashIcon}
  noCloseOnClick
  onclick={() => {
    Dialog.confirm({
      title: '구분선 삭제',
      message: '정말 이 구분선을 삭제하시겠어요?',
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        try {
          await deleteEntities({ input: { entityIds: [entity.id] } });
          cache.invalidate({ __typename: 'Site', id: entity.site.id, $field: 'deletedEntities' });
          mixpanel.track('delete_entities', { totalCount: 1, via });
        } catch {
          Toast.error('삭제 중 오류가 발생했습니다');
        }
      },
    });
  }}
  variant="danger"
>
  삭제
</MenuItem>

<HorizontalDivider color="secondary" />

<div
  class={flex({
    flexDirection: 'column',
    gap: '4px',
    paddingX: '10px',
    paddingY: '4px',
    fontSize: '12px',
    color: 'text.hint',
    userSelect: 'none',
  })}
>
  <div>생성: {dayjs(divider.createdAt).formatAsDateTime()}</div>

  <IdCopyMenuItem id={divider.id} label="구분선 ID 복사" />
</div>
