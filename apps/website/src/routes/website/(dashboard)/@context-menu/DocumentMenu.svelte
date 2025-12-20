<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, MenuItem } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { EntityAvailability, EntityVisibility } from '@/enums';
  import Columns2Icon from '~icons/lucide/columns-2';
  import FileDownIcon from '~icons/lucide/file-down';
  import InfoIcon from '~icons/lucide/info';
  import Rows2Icon from '~icons/lucide/rows-2';
  import TrashIcon from '~icons/lucide/trash';
  import { graphql } from '$graphql';
  import { getSplitViewContext, getViewContext } from '../[slug]/@split-view/context.svelte';
  import DocumentPdfExportModal from './DocumentPdfExportModal.svelte';

  type Props = {
    document: {
      id: string;
      title: string;
      createdAt: string;
      updatedAt: string;
    };
    entity: {
      slug: string;
      visibility: EntityVisibility;
      availability: EntityAvailability;
    };
    via: 'tree' | 'editor';
  };

  let { document, entity, via }: Props = $props();

  const splitView = getSplitViewContext();
  const view = getViewContext();

  let pdfExportModalOpen = $state(false);

  const deleteDocument = graphql(`
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
        }
      }
    }
  `);

  const handleDelete = () => {
    Dialog.confirm({
      title: '문서 삭제',
      message: `정말 "${document.title}" 문서를 삭제하시겠어요?`,
      children: deleteDetailsView,
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        await deleteDocument({ documentId: document.id });
        mixpanel.track('delete_document', { via });
      },
    });
  };

  const handleAddSplitView = (direction: 'horizontal' | 'vertical') => {
    if (view) {
      splitView.addView(entity.slug, {
        viewId: view.id,
        direction,
        position: 'after',
      });
    } else {
      splitView.addViewAtRoot(entity.slug, direction);
    }

    mixpanel.track('add_split_view', { via, direction });
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

<MenuItem icon={Columns2Icon} onclick={() => handleAddSplitView('horizontal')}>오른쪽에 열기</MenuItem>
<MenuItem icon={Rows2Icon} onclick={() => handleAddSplitView('vertical')}>아래에 열기</MenuItem>

<HorizontalDivider color="secondary" />

<MenuItem icon={FileDownIcon} noCloseOnClick onclick={() => (pdfExportModalOpen = true)}>PDF로 내보내기</MenuItem>

<DocumentPdfExportModal
  documentId={document.id}
  onClose={() => (pdfExportModalOpen = false)}
  slug={entity.slug}
  {via}
  bind:open={pdfExportModalOpen}
/>

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
  <div class={css({ fontWeight: 'medium' })}>
    {#if entity.visibility === EntityVisibility.UNLISTED || entity.availability === EntityAvailability.UNLISTED}
      <span class={css({ color: 'accent.brand.default' })}>
        {#if entity.visibility === EntityVisibility.UNLISTED && entity.availability === EntityAvailability.UNLISTED}
          링크 조회/편집 가능 문서
        {:else if entity.visibility === EntityVisibility.UNLISTED}
          링크 조회 가능 문서
        {:else if entity.availability === EntityAvailability.UNLISTED}
          링크 편집 가능 문서
        {/if}
      </span>
    {:else}
      <span>비공개 문서</span>
    {/if}
  </div>

  <div>
    <div>생성: {dayjs(document.createdAt).formatAsDateTime()}</div>
    <div>수정: {dayjs(document.updatedAt).formatAsDateTime()}</div>
  </div>
</div>
