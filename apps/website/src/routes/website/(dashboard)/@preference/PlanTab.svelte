<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import { PLAN_FEATURES } from '@typie/ui/constants';
  import { replaceState } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import type { DashboardLayout_PreferenceModal_PlanTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_PlanTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_PlanTab_user on User {
        id

        subscription {
          id
          state

          plan {
            id
            name
          }
        }
      }
    `),
  );

  const hasActiveSubscription = $derived(
    $user.subscription?.state === 'ACTIVE' ||
      $user.subscription?.state === 'IN_GRACE_PERIOD' ||
      $user.subscription?.state === 'WILL_EXPIRE',
  );
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <!-- Tab Header -->
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default', marginBottom: '4px' })}>플랜</h1>
    <p class={css({ fontSize: '13px', color: 'text.subtle', lineHeight: '[1.6]' })}>
      처음엔 가볍게 시작하고, 필요할 땐 제한 없이 모든 기능을 사용할 수 있어요.
    </p>
  </div>

  <!-- Plan Comparison Section -->
  <div>
    <div class={grid({ columns: 2, gap: '16px' })}>
      <!-- BASIC ACCESS Plan -->
      <div
        class={flex({
          flexDirection: 'column',
          borderWidth: '1px',
          borderColor: 'border.subtle',
          borderRadius: '8px',
          padding: '24px',
          backgroundColor: 'surface.default',
        })}
      >
        <div class={flex({ flexDirection: 'column', gap: '4px', marginBottom: '20px' })}>
          <div class={css({ fontSize: '15px', fontWeight: 'semibold', color: 'text.default' })}>BASIC ACCESS</div>
          <div class={css({ fontSize: '13px', color: 'text.subtle' })}>부담 없이, 필요한 만큼만 써보세요.</div>
        </div>

        <div class={css({ fontSize: '20px', fontWeight: 'bold', color: 'text.default', marginBottom: '20px' })}>무료</div>

        {#if !hasActiveSubscription}
          <div
            class={css({
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              height: '32px',
              fontSize: '13px',
              fontWeight: 'semibold',
              color: 'text.disabled',
              backgroundColor: 'surface.muted',
              borderWidth: '1px',
              borderColor: 'border.subtle',
              borderRadius: '6px',
              marginBottom: '20px',
            })}
          >
            현재 이용중
          </div>
        {:else}
          <Button
            style={css.raw({ width: 'full', marginBottom: '20px' })}
            onclick={() => {
              replaceState('', { shallowRoute: '/preference/billing' });
            }}
            size="sm"
            variant="secondary"
          >
            다운그레이드
          </Button>
        {/if}

        <div class={css({ borderTopWidth: '1px', borderColor: 'border.subtle', paddingTop: '20px' })}>
          <ul class={flex({ flexDirection: 'column', gap: '12px' })}>
            {#each PLAN_FEATURES.basic as feature, index (index)}
              <li class={flex({ alignItems: 'flex-start', gap: '8px' })}>
                <Icon style={css.raw({ color: 'text.subtle', flexShrink: 0, marginTop: '2px' })} icon={feature.icon} size={14} />
                <span class={css({ fontSize: '13px', color: 'text.default', lineHeight: '[1.6]' })}>{feature.label}</span>
              </li>
            {/each}
          </ul>
        </div>
      </div>

      <!-- FULL ACCESS Plan -->
      <div
        class={flex({
          flexDirection: 'column',
          borderWidth: '1px',
          borderColor: 'border.subtle',
          borderRadius: '8px',
          padding: '24px',
          backgroundColor: 'surface.default',
        })}
      >
        <div class={flex({ flexDirection: 'column', gap: '4px', marginBottom: '20px' })}>
          <div class={css({ fontSize: '15px', fontWeight: 'semibold', color: 'text.default' })}>FULL ACCESS</div>
          <div class={css({ fontSize: '13px', color: 'text.subtle' })}>더 많은 도구와 함께, 자유롭게 글을 시작해보세요.</div>
        </div>

        <div class={flex({ alignItems: 'baseline', gap: '4px', marginBottom: '20px' })}>
          <span class={css({ fontSize: '20px', fontWeight: 'bold', color: 'text.default' })}>4,900</span>
          <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.subtle' })}>원 / 월</span>
        </div>

        {#if hasActiveSubscription}
          <div
            class={css({
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              height: '32px',
              fontSize: '13px',
              fontWeight: 'semibold',
              color: 'text.disabled',
              backgroundColor: 'surface.muted',
              borderWidth: '1px',
              borderColor: 'border.subtle',
              borderRadius: '6px',
              marginBottom: '20px',
            })}
          >
            현재 이용중
          </div>
        {:else}
          <Button
            style={css.raw({ width: 'full', marginBottom: '20px' })}
            onclick={() => {
              replaceState('', { shallowRoute: '/preference/billing' });
            }}
            size="sm"
            variant="primary"
          >
            업그레이드
          </Button>
        {/if}

        <div class={css({ borderTopWidth: '1px', borderColor: 'border.subtle', paddingTop: '20px' })}>
          <ul class={flex({ flexDirection: 'column', gap: '12px' })}>
            {#each PLAN_FEATURES.full as feature, index (index)}
              <li class={flex({ alignItems: 'flex-start', gap: '8px' })}>
                <Icon style={css.raw({ color: 'text.subtle', flexShrink: 0, marginTop: '2px' })} icon={feature.icon} size={14} />
                <span class={css({ fontSize: '13px', color: 'text.default', lineHeight: '[1.6]' })}>{feature.label}</span>
              </li>
            {/each}
          </ul>
        </div>
      </div>
    </div>
  </div>
</div>
