<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import AlertTriangleIcon from '~icons/lucide/alert-triangle';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import { graphql } from '$mearie';
  import type { AdminImpersonateBanner_query$key } from '$mearie';

  type Props = {
    query$key: AdminImpersonateBanner_query$key;
  };

  let { query$key }: Props = $props();

  let confirmModalOpen = $state(false);

  const query = createFragment(
    graphql(`
      fragment AdminImpersonateBanner_query on Query {
        impersonation {
          admin {
            id
            name
            email
          }
          user {
            id
            name
            email
          }
        }
      }
    `),
    () => query$key,
  );

  const [adminStopImpersonation] = createMutation(
    graphql(`
      mutation AdminImpersonateBanner_AdminStopImpersonation_Mutation {
        adminStopImpersonation
      }
    `),
  );

  const handleStop = async () => {
    await adminStopImpersonation();
    location.assign('/admin');
  };
</script>

{#if query.data.impersonation}
  <div
    class={css({
      backgroundColor: 'accent.warning.subtle',
      borderBottomWidth: '1px',
      borderColor: 'accent.warning.default/30',
    })}
  >
    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', paddingX: '20px', paddingY: '10px' })}>
      <div class={flex({ alignItems: 'center', gap: '16px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'accent.warning.default' })} icon={AlertTriangleIcon} size={16} />
          <span class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'accent.warning.default' })}>대리 로그인 중</span>
        </div>

        <div class={flex({ alignItems: 'center', gap: '12px' })}>
          <div class={flex({ alignItems: 'center', gap: '6px' })}>
            <span class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'accent.warning.default' })}>
              {query.data.impersonation.admin.name}
            </span>
            <span class={css({ fontSize: '12px', color: 'accent.warning.default/70' })}>
              ({query.data.impersonation.admin.email})
            </span>
          </div>

          <Icon style={css.raw({ color: 'accent.warning.default/70' })} icon={ArrowRightIcon} size={14} />

          <div class={flex({ alignItems: 'center', gap: '6px' })}>
            <span class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'accent.warning.default' })}>
              {query.data.impersonation.user.name}
            </span>
            <span class={css({ fontSize: '12px', color: 'accent.warning.default/70' })}>
              ({query.data.impersonation.user.email})
            </span>
          </div>
        </div>
      </div>

      <Button onclick={() => (confirmModalOpen = true)} size="sm" variant="danger">중단</Button>
    </div>
  </div>

  <Modal style={css.raw({ padding: '24px', maxWidth: '400px' })} bind:open={confirmModalOpen}>
    <div class={flex({ flexDirection: 'column', gap: '24px' })}>
      <div class={flex({ flexDirection: 'column', gap: '8px' })}>
        <div class={css({ fontSize: '15px', fontWeight: 'bold', color: 'text.default' })}>대리 로그인을 중단할까요?</div>
        <div class={css({ fontSize: '13px', color: 'text.faint', wordBreak: 'keep-all' })}>
          현재 대상: {query.data.impersonation.user.name} ({query.data.impersonation.user.email})
        </div>
      </div>

      <div class={flex({ justifyContent: 'flex-end', gap: '10px' })}>
        <Button onclick={() => (confirmModalOpen = false)} size="sm" type="button" variant="secondary">취소</Button>
        <Button onclick={handleStop} size="sm" type="button" variant="danger">중단</Button>
      </div>
    </div>
  </Modal>
{/if}
