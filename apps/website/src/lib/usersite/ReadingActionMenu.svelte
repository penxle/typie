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
  import type { UsersiteReadingActionMenu_document$key } from '$mearie';

  type Props = {
    document$key: UsersiteReadingActionMenu_document$key;
  };

  let { document$key }: Props = $props();

  let reportDocumentOpen = $state(false);

  const fragment = createFragment(
    graphql(`
      fragment UsersiteReadingActionMenu_document on IEditorDocument {
        __typename
        id

        ... on DocumentView {
          id
          availableActions

          entity {
            id
            slug
          }
        }

        ... on PublicationView {
          id
          documentId
          editUrl
          availableActions
        }
      }
    `),
    () => document$key,
  );

  const [reportDocument] = createMutation(
    graphql(`
      mutation UsersiteReadingActionMenu_ReportDocument_Mutation($input: ReportDocumentInput!) {
        reportDocument(input: $input)
      }
    `),
  );

  const document = $derived(fragment.data.__typename === 'Document' ? null : fragment.data);
  const documentId = $derived(document?.__typename === 'PublicationView' ? document.documentId : document?.id);
  const editHref = $derived.by(() => {
    if (!document || !document.availableActions.includes('EDIT')) return null;
    if (document.__typename === 'PublicationView') return document.editUrl ?? null;
    return `${env.PUBLIC_WEBSITE_URL}/${document.entity.slug}`;
  });

  const form = createForm({
    schema: z.object({
      reason: z.string().optional(),
    }),
    onSubmit: async (data) => {
      if (!documentId) return;

      await reportDocument({
        input: {
          documentId,
          reason: data.reason,
        },
      });

      mixpanel.track('report_document');
      Toast.success('신고가 접수되었습니다');
      reportDocumentOpen = false;
    },
  });

  $effect(() => {
    void form;
  });
</script>

{#if document}
  <Menu
    style={css.raw({
      borderRadius: '6px',
      padding: '7px',
      color: 'text.muted',
      _hover: { backgroundColor: 'surface.hover' },
    })}
    placement="bottom-start"
  >
    {#snippet button()}
      <Icon icon={EllipsisVerticalIcon} size={18} />
    {/snippet}

    {#if editHref}
      <MenuItem external href={editHref} icon={PencilLineIcon} type="link">문서 수정</MenuItem>
    {:else}
      <MenuItem icon={MessageSquareWarningIcon} onclick={() => (reportDocumentOpen = true)}>문서 신고</MenuItem>
    {/if}
  </Menu>

  <Modal
    style={css.raw({ gap: '24px', padding: '20px', maxWidth: '500px' })}
    focusTrapOptions={{ initialFocus: '#reason' }}
    bind:open={reportDocumentOpen}
  >
    <p class={css({ fontWeight: 'medium', textAlign: 'center' })}>문서 신고</p>

    <form class={css({ display: 'flex', flexDirection: 'column', gap: '8px' })} onsubmit={form.handleSubmit}>
      <label class={css({ fontSize: '14px' })} for="reason">
        신고 사유
        <span class={css({ fontSize: '12px', color: 'text.muted' })}>(선택)</span>
      </label>

      <textarea
        id="reason"
        class={css({
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '8px',
          paddingX: '12px',
          paddingY: '10px',
          fontSize: '15px',
          resize: 'none',
          _hover: { borderColor: 'border.emphasis' },
          _focus: { borderColor: 'accent.default' },
        })}
        placeholder="신고 사유를 적어주세요"
        rows="3"
        bind:value={form.fields.reason}></textarea>

      <Button size="lg" type="submit">신고하기</Button>
    </form>
  </Modal>
{/if}
