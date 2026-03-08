<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { DocumentType, EntityAvailability, EntityVisibility } from '@/enums';
  import { TypieError } from '@/errors';
  import BlendIcon from '~icons/lucide/blend';
  import Columns2Icon from '~icons/lucide/columns-2';
  import CopyIcon from '~icons/lucide/copy';
  import DotIcon from '~icons/lucide/dot';
  import DownloadIcon from '~icons/lucide/download';
  import GlobeIcon from '~icons/lucide/globe';
  import InfoIcon from '~icons/lucide/info';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import Rows2Icon from '~icons/lucide/rows-2';
  import TrashIcon from '~icons/lucide/trash';
  import { goto } from '$app/navigation';
  import { unwrapError } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { getPane, getPaneGroup } from '../[slug]/@pane/context.svelte';
  import DocumentPdfExportModal from './DocumentPdfExportModal.svelte';
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
      url: string;
      visibility: EntityVisibility;
      availability: EntityAvailability;
    };
    via: 'tree' | 'editor';
    children?: Snippet;
  };

  let { document, entity, via, children }: Props = $props();

  const app = getAppContext();
  const paneGroup = getPaneGroup();
  const pane = getPane();

  let pdfExportModalOpen = $state(false);
  let docxExporting = $state(false);

  const [exportDocumentAsDocx] = createMutation(
    graphql(`
      mutation DocumentMenu_ExportDocumentAsDocx_Mutation($input: ExportDocumentAsDocxInput!) {
        exportDocumentAsDocx(input: $input) {
          data
          filename
        }
      }
    `),
  );

  const handleDocxExport = async () => {
    try {
      docxExporting = true;
      const result = await exportDocumentAsDocx({ input: { documentId: document.id } });
      const blob = new Blob([Uint8Array.fromBase64(result.exportDocumentAsDocx.data)], {
        type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', // cspell:disable-line
      });
      const url = URL.createObjectURL(blob);

      const a = globalThis.document.createElement('a');
      a.href = url;
      a.download = result.exportDocumentAsDocx.filename;
      a.click();

      URL.revokeObjectURL(url);
      mixpanel.track('export_document_docx', { via });
    } catch {
      Toast.error('DOCX 내보내기에 실패했어요. 잠시 후 다시 시도해주세요.');
    } finally {
      docxExporting = false;
    }
  };

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

  const handleDuplicate = async () => {
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

{#if document.documentType === DocumentType.NORMAL}
  <MenuItem icon={LayoutTemplateIcon} onclick={() => handleTypeChange(DocumentType.TEMPLATE)}>템플릿으로 전환</MenuItem>
{:else if document.documentType === DocumentType.TEMPLATE}
  <MenuItem icon={LayoutTemplateIcon} onclick={() => handleTypeChange(DocumentType.NORMAL)}>문서로 전환</MenuItem>
{/if}

{@render children?.()}

{#if app.preference.current.experimental_pdfExportEnabled || app.preference.current.experimental_docxExportEnabled}
  <HorizontalDivider color="secondary" />
{/if}

{#if app.preference.current.experimental_pdfExportEnabled}
  <MenuItem icon={DownloadIcon} noCloseOnClick onclick={() => (pdfExportModalOpen = true)}>PDF로 내보내기</MenuItem>
{/if}

{#if app.preference.current.experimental_docxExportEnabled}
  <MenuItem icon={DownloadIcon} loading={docxExporting} noCloseOnClick onclick={handleDocxExport}>DOCX로 내보내기</MenuItem>
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

  {#if document.characterCount !== undefined}
    <div>총 {comma(document.characterCount)}자</div>
  {/if}

  <div>
    <div>생성: {dayjs(document.createdAt).formatAsDateTime()}</div>
    <div>수정: {dayjs(document.updatedAt).formatAsDateTime()}</div>
  </div>
</div>
