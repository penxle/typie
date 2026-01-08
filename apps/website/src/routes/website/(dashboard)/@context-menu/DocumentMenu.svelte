<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { DocumentType, EntityAvailability, EntityVisibility } from '@/enums';
  import { TypieError } from '@/errors';
  import Columns2Icon from '~icons/lucide/columns-2';
  import CopyIcon from '~icons/lucide/copy';
  import DotIcon from '~icons/lucide/dot';
  import FileDownIcon from '~icons/lucide/file-down';
  import InfoIcon from '~icons/lucide/info';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import Rows2Icon from '~icons/lucide/rows-2';
  import TrashIcon from '~icons/lucide/trash';
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';
  import { getSplitViewContext, getViewContext } from '../[slug]/@split-view/context.svelte';
  import DocumentPdfExportModal from './DocumentPdfExportModal.svelte';

  type Props = {
    document: {
      id: string;
      title: string;
      documentType: DocumentType;
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

  const app = getAppContext();
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

  const duplicateDocument = graphql(`
    mutation DocumentMenu_DuplicateDocument_Mutation($input: DuplicateDocumentInput!) {
      duplicateDocument(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const updateDocumentType = graphql(`
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
  `);

  const handleDuplicate = async () => {
    try {
      const resp = await duplicateDocument({ documentId: document.id });
      mixpanel.track('duplicate_document', { via });
      await goto(`/${resp.entity.slug}`);
    } catch (err) {
      const errorMessages: Record<string, string> = {
        character_count_limit_exceeded: '현재 플랜의 글자 수 제한을 초과했어요.',
        blob_size_limit_exceeded: '현재 플랜의 파일 크기 제한을 초과했어요.',
      };

      if (err instanceof TypieError) {
        const message = errorMessages[err.code] || err.code;
        Toast.error(message);
      }
    }
  };

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

  const handleTypeChange = (newType: DocumentType) => {
    const isToTemplate = newType === DocumentType.TEMPLATE;

    Dialog.confirm({
      title: isToTemplate ? '템플릿으로 전환' : '문서로 전환',
      message: isToTemplate
        ? '이 문서를 템플릿으로 전환하시겠어요?\n앞으로 새 문서를 생성할 때 이 문서의 내용을 쉽게 이용할 수 있어요.'
        : '이 템플릿을 다시 일반 문서로 전환하시겠어요?',
      actionLabel: '전환',
      actionHandler: async () => {
        await updateDocumentType({ documentId: document.id, type: newType });
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

<MenuItem icon={CopyIcon} onclick={handleDuplicate}>복제</MenuItem>

{#if document.documentType === DocumentType.NORMAL}
  <MenuItem icon={LayoutTemplateIcon} onclick={() => handleTypeChange(DocumentType.TEMPLATE)}>템플릿으로 전환</MenuItem>
{:else if document.documentType === DocumentType.TEMPLATE}
  <MenuItem icon={LayoutTemplateIcon} onclick={() => handleTypeChange(DocumentType.NORMAL)}>문서로 전환</MenuItem>
{/if}

{#if app.preference.current.experimental_pdfExportEnabled}
  <HorizontalDivider color="secondary" />
{/if}

{#if app.preference.current.experimental_pdfExportEnabled}
  <MenuItem icon={FileDownIcon} noCloseOnClick onclick={() => (pdfExportModalOpen = true)}>PDF로 내보내기</MenuItem>
{/if}

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

  <div>
    <div>생성: {dayjs(document.createdAt).formatAsDateTime()}</div>
    <div>수정: {dayjs(document.updatedAt).formatAsDateTime()}</div>
  </div>
</div>
