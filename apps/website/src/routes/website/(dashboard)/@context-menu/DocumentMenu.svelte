<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { DocumentType, EntityAvailability, EntityVisibility } from '@typie/lib/enums';
  import { TypieError } from '@typie/lib/errors';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import BlendIcon from '~icons/lucide/blend';
  import ClipboardCopyIcon from '~icons/lucide/clipboard-copy';
  import ClipboardPasteIcon from '~icons/lucide/clipboard-paste';
  import Columns2Icon from '~icons/lucide/columns-2';
  import CopyIcon from '~icons/lucide/copy';
  import DotIcon from '~icons/lucide/dot';
  import FileDownIcon from '~icons/lucide/file-down';
  import GlobeIcon from '~icons/lucide/globe';
  import InfoIcon from '~icons/lucide/info';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import Rows2Icon from '~icons/lucide/rows-2';
  import ScissorsIcon from '~icons/lucide/scissors';
  import TrashIcon from '~icons/lucide/trash';
  import { goto } from '$app/navigation';
  import { cache, unwrapError } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { getPane, getPaneGroup } from '../[slug]/@pane/context.svelte';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import EntityIconPicker from './EntityIconPicker.svelte';
  import { showPasteToast } from './paste-toast';
  import type { Snippet } from 'svelte';

  type Props = {
    document: {
      id: string;
      title: string;
      documentType: DocumentType;
      characterCount?: number;
      createdAt: string;
      updatedAt: string;
    };
    entity: {
      id: string;
      slug: string;
      order?: string;
      url: string;
      visibility: EntityVisibility;
      availability: EntityAvailability;
      icon: string;
      iconColor: string;
      parent?: { id: string } | null;
    };
    via: 'tree' | 'editor';
    children?: Snippet;
  };

  let { document, entity, via, children }: Props = $props();

  const app = getAppContext();
  const paneGroup = getPaneGroup();
  const pane = getPane();

  const [deleteDocument] = createMutation(
    graphql(`
      mutation DocumentMenu_DeleteDocument_Mutation($input: DeleteDocumentInput!) {
        deleteDocument(input: $input) {
          id

          entity {
            id

            site {
              id
            }

            user {
              id

              recentlyViewedEntities {
                id
              }
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
      }
    `),
  );

  const [duplicateDocument] = createMutation(
    graphql(`
      mutation DocumentMenu_DuplicateDocument_Mutation($input: DuplicateDocumentInput!) {
        duplicateDocument(input: $input) {
          id

          entity {
            id
            slug

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

  const [updateDocumentType] = createMutation(
    graphql(`
      mutation DocumentMenu_UpdateDocumentType_Mutation($input: UpdateDocumentTypeInput!) {
        updateDocumentType(input: $input) {
          id
          type

          entity {
            id

            site {
              id

              documentTemplates {
                id
              }
            }
          }
        }
      }
    `),
  );

  const [moveEntities] = createMutation(
    graphql(`
      mutation DocumentMenu_MoveEntities_Mutation($input: MoveEntitiesInput!) {
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
      mutation DocumentMenu_CopyEntities_Mutation($input: CopyEntitiesInput!) {
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

  const [updateEntityIcon] = createMutation(
    graphql(`
      mutation DocumentMenu_UpdateEntityIcon_Mutation($input: UpdateEntityIconInput!) {
        updateEntityIcon(input: $input) {
          id
          icon
          iconColor
        }
      }
    `),
  );

  const getUpperOrder = () => {
    const el = globalThis.document.querySelector<HTMLElement>(`[data-id="${entity.id}"]`);
    if (!el) return;

    let nextEl = el.nextElementSibling as HTMLElement | null;
    while (nextEl && !Object.hasOwn(nextEl.dataset, 'id')) {
      nextEl = nextEl.nextElementSibling as HTMLElement | null;
    }
    return nextEl?.dataset.order;
  };

  const handleDuplicate = async () => {
    if (!SubscribeModal.gate('entity_duplicate')) {
      return;
    }

    try {
      const resp = await duplicateDocument({ input: { documentId: document.id } });
      mixpanel.track('duplicate_document', { via });
      await goto(`/${resp.duplicateDocument.entity.slug}`);
    } catch (err) {
      const errorMessages: Record<string, string> = {
        character_count_limit_exceeded: '현재 플랜의 글자 수 제한을 초과했어요.',
        blob_size_limit_exceeded: '현재 플랜의 파일 크기 제한을 초과했어요.',
      };

      const error = unwrapError(err);
      if (error instanceof TypieError) {
        const message = errorMessages[error.code] || error.code;
        Toast.error(message);
      }
    }
  };

  const findSiblingSlug = (slug: string): string | undefined => {
    const el = globalThis.document.querySelector<HTMLElement>(`[data-slug="${slug}"]`);
    if (!el) return;

    let next = el.nextElementSibling as HTMLElement | null;
    while (next && !next.dataset.slug) next = next.nextElementSibling as HTMLElement | null;
    if (next?.dataset.slug) return next.dataset.slug;

    let prev = el.previousElementSibling as HTMLElement | null;
    while (prev && !prev.dataset.slug) prev = prev.previousElementSibling as HTMLElement | null;
    return prev?.dataset.slug;
  };

  const handleDelete = () => {
    Dialog.confirm({
      title: '문서 삭제',
      message: `정말 "${document.title}" 문서를 삭제하시겠어요?`,
      children: deleteDetailsView,
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        const siblingSlug = findSiblingSlug(entity.slug);

        await deleteDocument({ input: { documentId: document.id } });
        mixpanel.track('delete_document', { via });

        const focusedPane = paneGroup.panes.find((p) => p.id === paneGroup.state.current.focusedPaneId);
        if (focusedPane?.kind !== 'entity' || focusedPane.slug !== entity.slug) return;

        if (paneGroup.panes.length > 1) {
          paneGroup.removePane(focusedPane.id);
        } else if (siblingSlug) {
          paneGroup.replacePane(focusedPane.id, { kind: 'entity', slug: siblingSlug });
        } else {
          paneGroup.replacePane(focusedPane.id, { kind: 'home' });
        }
      },
    });
  };

  const handleTypeChange = (newType: DocumentType) => {
    const isToTemplate = newType === DocumentType.TEMPLATE;

    Dialog.confirm({
      title: isToTemplate ? '템플릿으로 전환' : '문서로 전환',
      message: isToTemplate
        ? '이 문서를 템플릿으로 전환하시겠어요?\n앞으로 새 문서를 생성할 때 이 문서의 내용을 쉽게 이용할 수 있어요.'
        : '이 템플릿을 다시 일반 문서로 전환하시겠어요?',
      actionLabel: '전환',
      actionHandler: async () => {
        if (!SubscribeModal.gate('entity_update_type')) {
          return;
        }

        await updateDocumentType({ input: { documentId: document.id, type: newType } });
      },
    });
  };

  const handleAddPane = (direction: 'horizontal' | 'vertical') => {
    const targetPaneId = pane?.id ?? paneGroup.state.current.focusedPaneId;
    if (!targetPaneId) return;

    paneGroup.addPane(
      { kind: 'entity', slug: entity.slug },
      { paneId: targetPaneId, side: direction === 'horizontal' ? 'right' : 'bottom' },
    );
    mixpanel.track('add_pane', { via, direction });
  };
</script>

{#snippet deleteDetailsView()}
  <div
    class={flex({
      alignItems: 'center',
      gap: '6px',
      borderRadius: '8px',
      paddingX: '12px',
      paddingY: '8px',
      backgroundColor: 'surface.subtle',
    })}
  >
    <Icon style={css.raw({ color: 'text.muted' })} icon={InfoIcon} size={14} />
    <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.muted' })}>삭제 후 30일 동안 휴지통에 보관돼요</span>
  </div>
{/snippet}

<EntityIconPicker
  icon={entity.icon}
  iconColor={entity.iconColor}
  onColorSelect={async (color) => {
    if (!SubscribeModal.gate('entity_update_icon')) {
      return;
    }

    await updateEntityIcon(
      { input: { entityId: entity.id, icon: entity.icon, iconColor: color } },
      { metadata: { cache: { optimisticResponse: { updateEntityIcon: { id: entity.id, icon: entity.icon, iconColor: color } } } } },
    );
  }}
  onIconSelect={async (name) => {
    if (!SubscribeModal.gate('entity_update_icon')) {
      return;
    }

    await updateEntityIcon(
      { input: { entityId: entity.id, icon: name, iconColor: entity.iconColor } },
      { metadata: { cache: { optimisticResponse: { updateEntityIcon: { id: entity.id, icon: name, iconColor: entity.iconColor } } } } },
    );
  }}
/>

<HorizontalDivider color="secondary" />

<MenuItem icon={Columns2Icon} onclick={() => handleAddPane('horizontal')}>오른쪽에 열기</MenuItem>
<MenuItem icon={Rows2Icon} onclick={() => handleAddPane('vertical')}>아래에 열기</MenuItem>

<MenuItem external href={entity.url} icon={GlobeIcon} type="link">스페이스에서 열기</MenuItem>

<HorizontalDivider color="secondary" />

<MenuItem
  icon={BlendIcon}
  onclick={() => {
    app.state.shareOpen = [entity.id];
    if (via === 'editor') {
      mixpanel.track('open_document_share_modal', { via: 'editor' });
    }
  }}
>
  공유 및 게시
</MenuItem>

<MenuItem icon={CopyIcon} onclick={handleDuplicate}>복제</MenuItem>

{#if via === 'tree'}
  <MenuItem
    icon={ClipboardCopyIcon}
    onclick={() => {
      const currentSiteId = app.preference.current.currentSiteId;
      if (!currentSiteId) return;

      app.state.clipboard = {
        mode: 'copy',
        entityIds: [entity.id],
        sourceSiteId: currentSiteId,
      };
    }}
  >
    복사
  </MenuItem>

  <MenuItem
    icon={ScissorsIcon}
    onclick={() => {
      const currentSiteId = app.preference.current.currentSiteId;
      if (!currentSiteId) return;

      app.state.clipboard = {
        mode: 'cut',
        entityIds: [entity.id],
        sourceSiteId: currentSiteId,
      };
    }}
  >
    잘라내기
  </MenuItem>

  {#if app.state.clipboard && entity.order}
    <MenuItem
      icon={ClipboardPasteIcon}
      onclick={() => {
        const clipboard = app.state.clipboard;
        if (!clipboard) return;
        const currentSiteId = app.preference.current.currentSiteId;
        if (!currentSiteId) return;

        if (!SubscribeModal.gate('entity_paste')) {
          return;
        }

        const upperOrder = getUpperOrder() ?? null;
        const count = clipboard.entityIds.length;

        const promise = (async () => {
          if (clipboard.mode === 'cut') {
            const isCrossSite = clipboard.sourceSiteId !== currentSiteId;
            await moveEntities({
              input: {
                entityIds: clipboard.entityIds,
                parentEntityId: entity.parent?.id ?? null,
                lowerOrder: entity.order,
                upperOrder,
                ...(isCrossSite && { targetSiteId: currentSiteId }),
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
                targetSiteId: currentSiteId,
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
{/if}

{#if document.documentType === DocumentType.NORMAL}
  <MenuItem icon={LayoutTemplateIcon} onclick={() => handleTypeChange(DocumentType.TEMPLATE)}>템플릿으로 전환</MenuItem>
{:else if document.documentType === DocumentType.TEMPLATE}
  <MenuItem icon={LayoutTemplateIcon} onclick={() => handleTypeChange(DocumentType.NORMAL)}>문서로 전환</MenuItem>
{/if}

{@render children?.()}

<HorizontalDivider color="secondary" />

<MenuItem icon={FileDownIcon} onclick={() => (app.state.exportOpen = entity.slug)}>파일로 내보내기</MenuItem>

<HorizontalDivider color="secondary" />

<MenuItem icon={TrashIcon} onclick={handleDelete} variant="danger">삭제</MenuItem>

<HorizontalDivider color="secondary" />

<div
  class={flex({
    flexDirection: 'column',
    gap: '4px',
    paddingX: '10px',
    paddingY: '4px',
    fontSize: '12px',
    color: 'text.faint',
    userSelect: 'none',
  })}
>
  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <div class={css({ fontWeight: 'medium' })}>
      {#if entity.visibility === EntityVisibility.PUBLIC}
        <span class={css({ color: 'accent.success.default' })}>공개 조회</span>
      {:else if entity.visibility === EntityVisibility.UNLISTED}
        <span class={css({ color: 'accent.brand.default' })}>링크 조회</span>
      {:else}
        <span>비공개</span>
      {/if}
    </div>

    <Icon icon={DotIcon} size={12} />

    <div class={css({ fontWeight: 'medium' })}>
      {#if entity.availability === EntityAvailability.UNLISTED}
        <span class={css({ color: 'accent.brand.default' })}>링크 편집</span>
      {:else}
        <span>나만 편집</span>
      {/if}
    </div>
  </div>

  {#if document.characterCount !== undefined}
    <div>총 {comma(document.characterCount)}자</div>
  {/if}

  <div>
    <div>생성: {dayjs(document.createdAt).formatAsDateTime()}</div>
    <div>수정: {dayjs(document.updatedAt).formatAsDateTime()}</div>
  </div>

  <button
    class={flex({
      alignItems: 'center',
      gap: '2px',
      width: 'fit',
      cursor: 'pointer',
      fontSize: '11px',
      color: 'text.disabled',
      transition: 'common',
      _hover: { color: 'text.muted' },
    })}
    onclick={async () => {
      await navigator.clipboard.writeText(document.id);
      Toast.success('문서 ID가 복사되었어요');
    }}
    type="button"
  >
    <Icon icon={CopyIcon} size={12} />
    문서 ID 복사
  </button>
</div>
