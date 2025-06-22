<script lang="ts">
  import AlertTriangleIcon from '~icons/lucide/alert-triangle';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import { fragment, graphql } from '$graphql';
  import { AdminIcon } from '$lib/components/admin';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { AdminImpersonateBanner_query } from '$graphql';

  type Props = {
    $query: AdminImpersonateBanner_query;
  };

  let { $query: _query }: Props = $props();

  const query = fragment(
    _query,
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
  );

  const adminStopImpersonation = graphql(`
    mutation AdminImpersonateBanner_AdminStopImpersonation_Mutation {
      adminStopImpersonation
    }
  `);

  const handleStop = async () => {
    if (confirm('정말로 impersonate를 중지하시겠습니까?')) {
      try {
        await adminStopImpersonation();
        window.location.href = '/admin';
      } catch {
        alert('Failed to stop impersonation');
      }
    }
  };
</script>

{#if $query.impersonation}
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
          <AdminIcon style={{ color: 'gray.900' }} icon={AlertTriangleIcon} size={16} />
          <span class={css({ fontWeight: 'bold', color: 'gray.900' })}>IMPERSONATING</span>
        </div>

        <div class={flex({ alignItems: 'center', gap: '12px' })}>
          <div class={flex({ alignItems: 'center', gap: '6px' })}>
            <span class={css({ fontWeight: 'bold', color: 'gray.900' })}>
              {$query.impersonation.admin.name}
            </span>
            <span class={css({ color: 'gray.700', fontSize: '11px' })}>
              ({$query.impersonation.admin.email})
            </span>
          </div>

          <AdminIcon style={{ color: 'gray.700' }} icon={ArrowRightIcon} size={16} />

          <div class={flex({ alignItems: 'center', gap: '6px' })}>
            <span class={css({ fontWeight: 'bold', color: 'gray.900' })}>
              {$query.impersonation.user.name}
            </span>
            <span class={css({ color: 'gray.700', fontSize: '11px' })}>
              ({$query.impersonation.user.email})
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
        onclick={handleStop}
        type="button"
      >
        STOP IMPERSONATION
      </button>
    </div>
  </div>
{/if}
