<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { pushState } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { getAppContext } from '$lib/context';
  import { comma, formatBytes } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { DashboardLayout_PlanUsageWidget_site, DashboardLayout_PlanUsageWidget_user } from '$graphql';

  type Props = {
    $site: DashboardLayout_PlanUsageWidget_site;
    $user: DashboardLayout_PlanUsageWidget_user;
  };

  let { $site: _site, $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PlanUsageWidget_user on User {
        id

        planRule {
          maxTotalCharacterCount
          maxTotalBlobSize
        }

        plan {
          id
        }
      }
    `),
  );

  const site = fragment(
    _site,
    graphql(`
      fragment DashboardLayout_PlanUsageWidget_site on Site {
        id

        usage {
          totalCharacterCount
          totalBlobSize
        }
      }
    `),
  );

  const app = getAppContext();

  const totalCharacterCountProgress = $derived.by(() => {
    if ($user.planRule.maxTotalCharacterCount === -1) {
      return -1;
    }

    return Math.min(1, $site.usage.totalCharacterCount / $user.planRule.maxTotalCharacterCount);
  });

  const totalBlobSizeProgress = $derived.by(() => {
    if ($user.planRule.maxTotalBlobSize === -1) {
      return -1;
    }

    return Math.min(1, $site.usage.totalBlobSize / $user.planRule.maxTotalBlobSize);
  });

  $effect(() => {
    app.state.progress.totalCharacterCount = totalCharacterCountProgress;
    app.state.progress.totalBlobSize = totalBlobSizeProgress;
  });
</script>

{#if totalCharacterCountProgress !== -1 || totalBlobSizeProgress !== -1}
  <button
    class={flex({
      flexDirection: 'column',
      gap: '8px',
      position: 'sticky',
      bottom: '0',
      zIndex: '50',
      borderTopWidth: '1px',
      borderColor: 'gray.100',
      paddingX: '12px',
      paddingTop: '12px',
      paddingBottom: '20px',
      backgroundColor: 'white',
      transitionProperty: 'background-color',
      transitionDuration: '250ms',
      transitionTimingFunction: 'ease',
      _hover: { backgroundColor: 'gray.50' },
    })}
    onclick={() => {
      pushState('', { shallowRoute: '/preference/billing' });
      mixpanel.track('open_billing_tab', { via: 'usage_widget' });
    }}
    type="button"
  >
    <div class={flex({ flexDirection: 'column', gap: '2px', width: 'full' })}>
      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '2px' })}>
        <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'gray.500' })}>글자 수</div>

        <div class={css({ fontSize: '12px', color: 'gray.500' })}>
          {comma($site.usage.totalCharacterCount)}자 / {comma($user.planRule.maxTotalCharacterCount)}자
        </div>
      </div>

      <div class={css({ position: 'relative', borderRadius: 'full', height: '4px', overflow: 'hidden' })}>
        <div
          style:width={`${totalCharacterCountProgress * 100}%`}
          class={css({
            position: 'absolute',
            left: '0',
            insetY: '0',
            borderRightRadius: 'full',
            backgroundColor: 'brand.400',
            maxWidth: 'full',
            transitionProperty: 'width',
            transitionDuration: '150ms',
            transitionTimingFunction: 'ease',
          })}
        ></div>

        <div class={css({ backgroundColor: 'gray.200', height: 'full' })}></div>
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '2px', width: 'full' })}>
      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '4px' })}>
        <div class={css({ fontSize: '12px', color: 'gray.700' })}>파일 업로드</div>

        <div class={css({ fontSize: '12px', color: 'gray.500' })}>
          {formatBytes($site.usage.totalBlobSize)} / {formatBytes($user.planRule.maxTotalBlobSize)}
        </div>
      </div>

      <div class={css({ position: 'relative', borderRadius: 'full', height: '4px', overflow: 'hidden' })}>
        <div
          style:width={`${totalBlobSizeProgress * 100}%`}
          class={css({
            position: 'absolute',
            left: '0',
            insetY: '0',
            borderRightRadius: 'full',
            backgroundColor: 'brand.400',
            maxWidth: 'full',
            transitionProperty: 'width',
            transitionDuration: '150ms',
            transitionTimingFunction: 'ease',
          })}
        ></div>

        <div class={css({ backgroundColor: 'gray.200', height: 'full' })}></div>
      </div>
    </div>
  </button>
{/if}
