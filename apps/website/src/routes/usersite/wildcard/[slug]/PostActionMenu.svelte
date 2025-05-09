<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import EllipsisVerticalIcon from '~icons/lucide/ellipsis-vertical';
  import { env } from '$env/dynamic/public';
  import { fragment, graphql } from '$graphql';
  import { Button, Icon, Menu, MenuItem, Modal } from '$lib/components';
  import { createForm } from '$lib/form';
  import { Toast } from '$lib/notification';
  import { css } from '$styled-system/css';
  import type { UsersiteWildcardSlugPage_PostActionMenu_entityView } from '$graphql';

  type Props = {
    $entityView: UsersiteWildcardSlugPage_PostActionMenu_entityView;
  };

  let { $entityView: _entityView }: Props = $props();

  let reportPostOpen = $state(false);

  const entityView = fragment(
    _entityView,
    graphql(`
      fragment UsersiteWildcardSlugPage_PostActionMenu_entityView on EntityView {
        id
        slug

        node {
          __typename

          ... on PostView {
            id
            availableActions
          }
        }
      }
    `),
  );

  const reportPost = graphql(`
    mutation UsersiteWildcardSlugPage_PostActionMenu_ReportPost_Mutation($input: ReportPostInput!) {
      reportPost(input: $input)
    }
  `);

  const form = createForm({
    schema: z.object({
      reason: z.string().optional(),
    }),
    onSubmit: async (data) => {
      if ($entityView.node.__typename !== 'PostView') return;

      await reportPost({
        postId: $entityView.node.id,
        reason: data.reason,
      });

      mixpanel.track('report_post');
      Toast.success('신고가 접수되었습니다');
      reportPostOpen = false;
    },
  });
</script>

{#if $entityView.node.__typename === 'PostView'}
  <Menu
    style={css.raw({
      borderRadius: '4px',
      padding: '3px',
      _hover: { backgroundColor: 'gray.100' },
    })}
    placement="bottom-start"
  >
    {#snippet button()}
      <Icon icon={EllipsisVerticalIcon} size={18} />
    {/snippet}

    {#if $entityView.node.availableActions.includes('EDIT')}
      <MenuItem external href={`${env.PUBLIC_WEBSITE_URL}/${$entityView.slug}`} type="link">포스트 수정</MenuItem>
    {:else}
      <MenuItem onclick={() => (reportPostOpen = true)}>포스트 신고</MenuItem>
    {/if}
  </Menu>

  <Modal style={css.raw({ gap: '24px', padding: '20px', maxWidth: '500px' })} bind:open={reportPostOpen}>
    <p class={css({ fontWeight: 'medium', textAlign: 'center' })}>포스트 신고</p>

    <form class={css({ display: 'flex', flexDirection: 'column', gap: '8px' })} onsubmit={form.handleSubmit}>
      <label class={css({ fontSize: '14px' })} for="reason">
        신고 사유
        <span class={css({ fontSize: '12px', color: 'gray.400' })}>(선택)</span>
      </label>

      <textarea
        id="reason"
        class={css({
          borderWidth: '1px',
          borderColor: 'gray.300',
          borderRadius: '8px',
          paddingX: '12px',
          paddingY: '10px',
          fontSize: '15px',
          resize: 'none',
          _hover: { borderColor: 'brand.400' },
          _focus: { borderColor: 'brand.600' },
        })}
        placeholder="신고 사유를 적어주세요"
        rows="3"
        bind:value={form.fields.reason}
      ></textarea>

      <Button size="lg" type="submit">신고하기</Button>
    </form>
  </Modal>
{/if}
