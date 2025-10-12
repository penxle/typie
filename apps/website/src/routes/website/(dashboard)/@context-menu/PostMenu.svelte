<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { comma, downloadFromBase64 } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { EntityAvailability, EntityVisibility, ExportLayoutMode, PostLayoutMode } from '@/enums';
  import { TypieError } from '@/errors';
  import BlendIcon from '~icons/lucide/blend';
  import Columns2Icon from '~icons/lucide/columns-2';
  import CopyIcon from '~icons/lucide/copy';
  import DownloadIcon from '~icons/lucide/download';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import InfoIcon from '~icons/lucide/info';
  import Rows2Icon from '~icons/lucide/rows-2';
  import TrashIcon from '~icons/lucide/trash';
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';
  import { getPostYjsAttrs } from '$lib/utils/yjs-post';
  import { getSplitViewContext, getViewContext } from '../[slug]/@split-view/context.svelte';
  import PdfExportModal from './PdfExportModal.svelte';
  import type { PageLayout } from '@typie/ui/utils';
  import type { Snippet } from 'svelte';

  type Props = {
    post: {
      id: string;
      title: string;
      characterCount?: number;
    };
    entity: {
      id: string;
      slug: string;
      url: string;
      visibility: EntityVisibility;
      availability: EntityAvailability;
    };
    via: 'tree' | 'editor';
    pageLayout?: PageLayout;
    layoutMode?: PostLayoutMode;
    children?: Snippet;
  };

  let { post, entity, via, pageLayout, layoutMode, children }: Props = $props();

  const app = getAppContext();
  const splitView = getSplitViewContext();
  const view = getViewContext();

  let showPdfExportModal = $state(false);
  let exportModalPageLayout = $state<PageLayout | undefined>();
  let exportModalPageEnabled = $state<boolean>(false);
  let loadingDocxExport = $state(false);

  const duplicatePost = graphql(`
    mutation PostMenu_DuplicatePost_Mutation($input: DuplicatePostInput!) {
      duplicatePost(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const deletePost = graphql(`
    mutation PostMenu_DeletePost_Mutation($input: DeletePostInput!) {
      deletePost(input: $input) {
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

  const exportPostAsPdf = graphql(`
    mutation PostMenu_ExportPostAsPdf_Mutation($input: ExportPostAsPdfInput!) {
      exportPostAsPdf(input: $input) {
        data
        filename
      }
    }
  `);

  const exportPostAsDocx = graphql(`
    mutation PostMenu_ExportPostAsDocx_Mutation($input: ExportPostAsDocxInput!) {
      exportPostAsDocx(input: $input) {
        data
        filename
      }
    }
  `);

  const handleDuplicate = async () => {
    try {
      const resp = await duplicatePost({ postId: post.id });
      mixpanel.track('duplicate_post', { via });
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
      title: '포스트 삭제',
      message: `정말 "${post.title}" 포스트를 삭제하시겠어요?`,
      children: deleteDetailsView,
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        await deletePost({ postId: post.id });
        mixpanel.track('delete_post', { via });
      },
    });
  };

  const handlePdfExport = async () => {
    let layout = pageLayout;

    if (via === 'tree') {
      const attrs = await getPostYjsAttrs<{
        pageLayout: PageLayout;
        layoutMode: PostLayoutMode;
      }>(post.id, ['pageLayout', 'layoutMode']);

      layout = attrs.pageLayout;
      layoutMode = attrs.layoutMode;
    }

    exportModalPageLayout = layout;
    exportModalPageEnabled = layoutMode === PostLayoutMode.PAGE;
    showPdfExportModal = true;
  };

  const handlePdfExportConfirm = async (layoutMode: ExportLayoutMode, pageLayout: PageLayout) => {
    try {
      const resp = await exportPostAsPdf({
        entityId: entity.id,
        layoutMode,
        ...pageLayout,
      });

      downloadFromBase64(resp.data, resp.filename, 'application/pdf');

      Toast.success('PDF 내보내기가 완료되었어요');
      mixpanel.track('export_post', { via, format: 'PDF', layoutMode });
    } catch {
      Toast.error('내보내기 중 오류가 발생했습니다');
    }
  };

  const handleDocxExport = async () => {
    loadingDocxExport = true;
    try {
      const resp = await exportPostAsDocx({
        entityId: entity.id,
      });

      // cspell:ignore wordprocessingml
      downloadFromBase64(resp.data, resp.filename, 'application/vnd.openxmlformats-officedocument.wordprocessingml.document');

      Toast.success('DOCX 내보내기가 완료되었어요');
      mixpanel.track('export_post', { via, format: 'DOCX' });
    } catch {
      Toast.error('내보내기 중 오류가 발생했습니다');
    } finally {
      loadingDocxExport = false;
    }
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

<MenuItem external href={entity.url} icon={ExternalLinkIcon} type="link">사이트에서 열기</MenuItem>

<HorizontalDivider color="secondary" />

<MenuItem
  icon={BlendIcon}
  onclick={() => {
    app.state.shareOpen = [entity.id];
    if (via === 'editor') {
      mixpanel.track('open_post_share_modal', { via: 'editor' });
    }
  }}
>
  공유 및 게시
</MenuItem>

<MenuItem icon={CopyIcon} onclick={handleDuplicate}>복제</MenuItem>

{@render children?.()}

{#if app.preference.current.experimental_pdfExportEnabled || app.preference.current.experimental_docxExportEnabled}
  <HorizontalDivider color="secondary" />
{/if}

{#if app.preference.current.experimental_pdfExportEnabled}
  <MenuItem icon={DownloadIcon} noCloseOnClick onclick={handlePdfExport}>PDF로 내보내기</MenuItem>
{/if}

{#if app.preference.current.experimental_docxExportEnabled}
  <MenuItem icon={DownloadIcon} loading={loadingDocxExport} noCloseOnClick onclick={handleDocxExport}>DOCX로 내보내기</MenuItem>
{/if}

<HorizontalDivider color="secondary" />

<MenuItem icon={TrashIcon} onclick={handleDelete} variant="danger">삭제</MenuItem>

<PdfExportModal
  currentPageEnabled={exportModalPageEnabled}
  currentPageLayout={exportModalPageLayout}
  onClose={() => {
    showPdfExportModal = false;
  }}
  onConfirm={handlePdfExportConfirm}
  bind:open={showPdfExportModal}
/>

{#if via === 'tree'}
  <HorizontalDivider color="secondary" />

  <div class={css({ paddingX: '10px', paddingY: '4px', fontSize: '12px', color: 'text.disabled', userSelect: 'none' })}>
    <div class={css({ fontWeight: 'medium' })}>
      {#if entity.visibility === EntityVisibility.UNLISTED || entity.availability === EntityAvailability.UNLISTED}
        <span class={css({ color: 'accent.brand.default' })}>
          {#if entity.visibility === EntityVisibility.UNLISTED && entity.availability === EntityAvailability.UNLISTED}
            링크 조회/편집 가능 포스트
          {:else if entity.visibility === EntityVisibility.UNLISTED}
            링크 조회 가능 포스트
          {:else if entity.availability === EntityAvailability.UNLISTED}
            링크 편집 가능 포스트
          {/if}
        </span>
      {:else}
        <span>비공개 포스트</span>
      {/if}
    </div>

    {#if post.characterCount !== undefined}
      <span>총 {comma(post.characterCount)}자</span>
    {/if}
  </div>
{/if}
