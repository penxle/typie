<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { comma, downloadFromBase64 } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { EntityAvailability, EntityVisibility, ExportLayoutMode, PostLayoutMode, PostType } from '@/enums';
  import { TypieError } from '@/errors';
  import BlendIcon from '~icons/lucide/blend';
  import CopyIcon from '~icons/lucide/copy';
  import DownloadIcon from '~icons/lucide/download';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import InfoIcon from '~icons/lucide/info';
  import ShapesIcon from '~icons/lucide/shapes';
  import TrashIcon from '~icons/lucide/trash';
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';
  import { getPostYjsAttrs } from '$lib/utils/yjs-post';
  import PdfExportModal from './PdfExportModal.svelte';
  import type { PageLayout } from '@typie/ui/utils';

  type Props = {
    post: {
      id: string;
      title: string;
      type: PostType;
      characterCount?: number;
    };
    entity: {
      id: string;
      url: string;
      visibility: EntityVisibility;
      availability: EntityAvailability;
    };
    via: 'tree' | 'editor';
    pageLayout?: PageLayout;
    pageLayoutEnabled?: boolean;
  };

  let { post, entity, via, pageLayout, pageLayoutEnabled }: Props = $props();

  let showPdfExportModal = $state(false);
  let exportModalPageLayout = $state<PageLayout | undefined>();
  let exportModalPageEnabled = $state<boolean>(false);

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

  const updatePostType = graphql(`
    mutation PostMenu_UpdatePostType_Mutation($input: UpdatePostTypeInput!) {
      updatePostType(input: $input) {
        id
        type

        entity {
          id

          site {
            id

            templates {
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

  const app = getAppContext();

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

  const handleTypeChange = (newType: PostType) => {
    const isToTemplate = newType === PostType.TEMPLATE;

    Dialog.confirm({
      title: isToTemplate ? '템플릿으로 전환' : '포스트로 전환',
      message: isToTemplate
        ? '이 포스트를 템플릿으로 전환하시겠어요?\n앞으로 새 포스트를 생성할 때 이 포스트의 서식을 쉽게 이용할 수 있어요.'
        : '이 템플릿을 다시 일반 포스트로 전환하시겠어요?',
      actionLabel: '전환',
      actionHandler: async () => {
        await updatePostType({ postId: post.id, type: newType });
      },
    });
  };

  const handleExport = async () => {
    let layout = pageLayout;
    let pageEnabled = pageLayoutEnabled;

    if (!layout && via === 'tree') {
      const attrs = await getPostYjsAttrs<{
        pageLayout: PageLayout;
        layoutMode: PostLayoutMode;
      }>(post.id, ['pageLayout', 'layoutMode']);

      layout = attrs.pageLayout;
      pageEnabled = attrs.layoutMode === PostLayoutMode.PAGE;
    }

    exportModalPageLayout = layout;
    exportModalPageEnabled = !!pageEnabled;
    showPdfExportModal = true;
  };

  const handleExportConfirm = async (layoutMode: ExportLayoutMode, pageLayout: PageLayout) => {
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

{#if post.type === PostType.NORMAL}
  <MenuItem icon={ShapesIcon} onclick={() => handleTypeChange(PostType.TEMPLATE)}>템플릿으로 전환</MenuItem>
{:else if post.type === PostType.TEMPLATE}
  <MenuItem icon={ShapesIcon} onclick={() => handleTypeChange(PostType.NORMAL)}>포스트로 전환</MenuItem>
{/if}

{#if app.preference.current.experimental_pdfExportEnabled}
  <HorizontalDivider color="secondary" />

  <MenuItem icon={DownloadIcon} noCloseOnClick onclick={handleExport}>PDF로 내보내기</MenuItem>
{/if}

<HorizontalDivider color="secondary" />

<MenuItem icon={TrashIcon} onclick={handleDelete} variant="danger">삭제</MenuItem>

<PdfExportModal
  currentPageEnabled={exportModalPageEnabled}
  currentPageLayout={exportModalPageLayout}
  onClose={() => {
    showPdfExportModal = false;
  }}
  onConfirm={handleExportConfirm}
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
