<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import AlertTriangleIcon from '~icons/lucide/alert-triangle';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import { AdminIcon, AdminModal } from '$lib/components/admin';
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
    location.href = '/admin';
  };
</script>

{#if query.data.impersonation}
  <div
    class={css({
      backgroundColor: 'amber.500',
      fontFamily: 'mono',
      fontSize: '12px',
      letterSpacing: '0.02em',
    })}
  >
    <div
      class={flex({
        alignItems: 'center',
        justifyContent: 'space-between',
        paddingX: '20px',
        paddingY: '8px',
      })}
    >
      <div class={flex({ alignItems: 'center', gap: '16px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <AdminIcon style={css.raw({ color: 'gray.900' })} icon={AlertTriangleIcon} size={16} />
          <span class={css({ fontWeight: 'bold', color: 'gray.900' })}>IMPERSONATING</span>
        </div>

        <div class={flex({ alignItems: 'center', gap: '12px' })}>
          <div class={flex({ alignItems: 'center', gap: '6px' })}>
            <span class={css({ fontWeight: 'bold', color: 'gray.900' })}>
              {query.data.impersonation.admin.name}
            </span>
            <span class={css({ color: 'gray.700', fontSize: '11px' })}>
              ({query.data.impersonation.admin.email})
            </span>
          </div>

          <AdminIcon style={css.raw({ color: 'gray.700' })} icon={ArrowRightIcon} size={16} />

          <div class={flex({ alignItems: 'center', gap: '6px' })}>
            <span class={css({ fontWeight: 'bold', color: 'gray.900' })}>
              {query.data.impersonation.user.name}
            </span>
            <span class={css({ color: 'gray.700', fontSize: '11px' })}>
              ({query.data.impersonation.user.email})
            </span>
          </div>
        </div>
      </div>

      <button
        class={css({
          paddingX: '18px',
          paddingY: '6px',
          fontSize: '11px',
          fontWeight: 'medium',
          color: 'amber.500',
          backgroundColor: 'gray.900',
          borderWidth: '1px',
          borderColor: 'gray.900',
          cursor: 'pointer',
          transition: 'common',
          _hover: {
            backgroundColor: 'amber.500',
            color: 'gray.900',
            borderColor: 'gray.900',
          },
        })}
        onclick={() => (confirmModalOpen = true)}
        type="button"
      >
        STOP IMPERSONATION
      </button>
    </div>
  </div>

  <AdminModal
    actions={{
      cancel: {},
      confirm: {
        label: 'CONFIRM STOP',
        onclick: handleStop,
        variant: 'danger',
      },
    }}
    title="CONFIRM ACTION"
    bind:open={confirmModalOpen}
  >
    <div class={css({ marginBottom: '16px' })}>
      <p class={css({ marginBottom: '8px' })}>ARE YOU SURE YOU WANT TO STOP IMPERSONATING?</p>
      <p class={css({ color: 'amber.400' })}>
        CURRENT USER: {query.data.impersonation?.user.name.toUpperCase()} ({query.data.impersonation?.user.email})
      </p>
    </div>
  </AdminModal>
{/if}
