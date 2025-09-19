<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { getAppContext } from '@typie/ui/context';
  import { comma, formatBytes } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { defaultPlanRules } from '@/const';
  import { pushState } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
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

        subscription {
          id

          plan {
            id

            rule {
              maxTotalCharacterCount
              maxTotalBlobSize
            }
          }
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

  const planRule = $derived($user.subscription?.plan?.rule ?? defaultPlanRules);

  const totalCharacterCountProgress = $derived.by(() => {
    if (planRule.maxTotalCharacterCount === -1) {
      return -1;
    }

    return Math.min(1, $site.usage.totalCharacterCount / planRule.maxTotalCharacterCount);
  });

  const totalBlobSizeProgress = $derived.by(() => {
    if (planRule.maxTotalBlobSize === -1) {
      return -1;
    }

    return Math.min(1, $site.usage.totalBlobSize / planRule.maxTotalBlobSize);
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
      borderTopWidth: '1px',
      borderColor: 'border.default',
      paddingX: '12px',
      paddingTop: '12px',
      paddingBottom: '20px',
      backgroundColor: 'surface.default',
      transitionProperty: '[background-color]',
      transitionDuration: '250ms',
      transitionTimingFunction: 'ease',
      _hover: { backgroundColor: 'surface.subtle' },
    })}
    onclick={() => {
      pushState('', { shallowRoute: '/preference/billing' });
      mixpanel.track('open_billing_tab', { via: 'usage_widget' });
    }}
    type="button"
  >
    <div class={flex({ flexDirection: 'column', gap: '2px', width: 'full' })}>
      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '2px' })}>
        <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>글자 수</div>

        <div class={css({ fontSize: '12px', color: 'text.faint' })}>
          {comma($site.usage.totalCharacterCount)}자 / {comma(planRule.maxTotalCharacterCount)}자
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
            backgroundColor: 'accent.brand.default',
            maxWidth: 'full',
            transitionProperty: '[width]',
            transitionDuration: '150ms',
            transitionTimingFunction: 'ease',
          })}
        ></div>

        <div class={css({ backgroundColor: 'interactive.hover', height: 'full' })}></div>
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '2px', width: 'full' })}>
      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '4px' })}>
        <div class={css({ fontSize: '12px', color: 'text.subtle' })}>파일 업로드</div>

        <div class={css({ fontSize: '12px', color: 'text.faint' })}>
          {formatBytes($site.usage.totalBlobSize)} / {formatBytes(planRule.maxTotalBlobSize)}
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
            backgroundColor: 'accent.brand.default',
            maxWidth: 'full',
            transitionProperty: '[width]',
            transitionDuration: '150ms',
            transitionTimingFunction: 'ease',
          })}
        ></div>

        <div class={css({ backgroundColor: 'interactive.hover', height: 'full' })}></div>
      </div>
    </div>
  </button>
{/if}
