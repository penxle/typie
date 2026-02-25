<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Button, Icon, Menu, MenuItem, Modal } from '@typie/ui/components';
  import { createForm } from '@typie/ui/form';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import EllipsisVerticalIcon from '~icons/lucide/ellipsis-vertical';
  import MessageSquareWarningIcon from '~icons/lucide/message-square-warning';
  import PencilLineIcon from '~icons/lucide/pencil-line';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$mearie';
  import type { UsersiteWildcardSlugPage_PostActionMenu_entityView$key } from '$mearie';

  type Props = {
    entityView$key: UsersiteWildcardSlugPage_PostActionMenu_entityView$key;
  };

  let { entityView$key }: Props = $props();

  let reportPostOpen = $state(false);

  const entityView = createFragment(
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
    () => entityView$key,
  );

  const [reportPost] = createMutation(
    graphql(`
      mutation UsersiteWildcardSlugPage_PostActionMenu_ReportPost_Mutation($input: ReportPostInput!) {
        reportPost(input: $input)
      }
    `),
  );

  const form = createForm({
    schema: z.object({
      reason: z.string().optional(),
    }),
    onSubmit: async (data) => {
      if (entityView.data.node.__typename !== 'PostView') return;

      await reportPost({
        input: {
          postId: entityView.data.node.id,
          reason: data.reason,
        },
      });

      mixpanel.track('report_post');
      Toast.success('신고가 접수되었습니다');
      reportPostOpen = false;
    },
  });

  $effect(() => {
    void form;
  });
</script>

{#if entityView.data.node.__typename === 'PostView'}
  <Menu
    style={css.raw({
      borderRadius: '4px',
      padding: '3px',
      _hover: { backgroundColor: 'surface.muted' },
    })}
    placement="bottom-start"
  >
    {#snippet button()}
      <Icon icon={EllipsisVerticalIcon} size={18} />
    {/snippet}

    {#if entityView.data.node.availableActions.includes('EDIT')}
      <MenuItem external href={`${env.PUBLIC_WEBSITE_URL}/${entityView.data.slug}`} icon={PencilLineIcon} type="link">포스트 수정</MenuItem>
    {:else}
      <MenuItem icon={MessageSquareWarningIcon} onclick={() => (reportPostOpen = true)}>포스트 신고</MenuItem>
    {/if}
  </Menu>

  <Modal style={css.raw({ gap: '24px', padding: '20px', maxWidth: '500px' })} bind:open={reportPostOpen}>
    <p class={css({ fontWeight: 'medium', textAlign: 'center' })}>포스트 신고</p>

    <form class={css({ display: 'flex', flexDirection: 'column', gap: '8px' })} onsubmit={form.handleSubmit}>
      <label class={css({ fontSize: '14px' })} for="reason">
        신고 사유
        <span class={css({ fontSize: '12px', color: 'text.disabled' })}>(선택)</span>
      </label>

      <textarea
        id="reason"
        class={css({
          borderWidth: '1px',
          borderColor: 'border.strong',
          borderRadius: '8px',
          paddingX: '12px',
          paddingY: '10px',
          fontSize: '15px',
          resize: 'none',
          _hover: { borderColor: 'border.strong' },
          _focus: { borderColor: 'accent.brand.default' },
        })}
        placeholder="신고 사유를 적어주세요"
        rows="3"
        bind:value={form.fields.reason}
      ></textarea>

      <Button size="lg" type="submit">신고하기</Button>
    </form>
  </Modal>
{/if}
