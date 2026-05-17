<script lang="ts">
  import { createSubscription } from '@mearie/svelte';
  import { defaultPlanRules } from '@typie/lib/const';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { Button, HorizontalDivider } from '@typie/ui/components';
  import { setupAppContext } from '@typie/ui/context';
  import { Updater } from '@typie/ui/notification';
  import { isMobileDevice } from '@typie/ui/utils';
  import stringify from 'fast-json-stable-stringify';
  import mixpanel from 'mixpanel-browser';
  import qs from 'query-string';
  import { onMount, untrack } from 'svelte';
  import { browser } from '$app/environment';
  import { goto } from '$app/navigation';
  import { updated } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { env } from '$env/dynamic/public';
  import { pollBootstrapAssertion } from '$lib/bootstrap';
  import { EnvironmentBanner } from '$lib/components';
  import { AdminImpersonateBanner } from '$lib/components/admin';
  import { preloadEditorWasm } from '$lib/editor/editor.svelte';
  import { hydrateQuery } from '$lib/graphql';
  import { initWasm } from '$lib/wasm.svelte';
  import { graphql } from '$mearie';
  import { setupPaneGroup } from './[slug]/@pane/context.svelte';
  import { setupEditorRegistry } from './[slug]/@pane/editor-registry.svelte';
  import DocumentExportModal from './@context-menu/DocumentExportModal.svelte';
  import Notes from './@notes/Notes.svelte';
  import PreferenceModal from './@preference/PreferenceModal.svelte';
  import ShareModal from './@share/ShareModal.svelte';
  import SiteSettingsModal from './@site-settings/SiteSettingsModal.svelte';
  import StatsModal from './@stats/StatsModal.svelte';
  import TrashModal from './@trash/TrashModal.svelte';
  import CommandPalette from './CommandPalette.svelte';
  import MarketingConsentModal from './MarketingConsentModal.svelte';
  import PlanUpgradeModal from './PlanUpgradeModal.svelte';
  import ReferralWelcomeModal from './ReferralWelcomeModal.svelte';
  import Shortcuts from './Shortcuts.svelte';
  import ShortcutsModal from './ShortcutsModal.svelte';
  import Sidebar from './Sidebar.svelte';
  import TrialExpiredModal from './TrialExpiredModal.svelte';
  import UserSurveyModal from './UserSurveyModal.svelte';

  if (browser) {
    preloadEditorWasm();
  }

  let { data, children } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const app = setupAppContext(query.data.me.id);

  let currentSite = $derived(query.data.me.sites.find((s) => s.id === app.preference.current.currentSiteId) ?? query.data.me.sites[0]);
  let siteId = $derived(currentSite.id);
  let userId = $derived(query.data.me.id);

  createSubscription(
    graphql(`
      subscription DashboardLayout_SiteUpdateStream($siteId: ID!) {
        siteUpdateStream(siteId: $siteId) {
          ... on Site {
            id

            ...DashboardLayout_EntityTree_site
          }

          ... on Entity {
            id
            state

            children {
              id
              ...DashboardLayout_EntityTree_Entity_entity
            }

            node {
              __typename

              ... on Folder {
                id
                name
              }

              ... on Document {
                id
                title
                nullableTitle
                subtitle

                characterCountChange {
                  additions
                  deletions
                }
              }
            }
          }
        }
      }
    `),
    () => ({ siteId }),
  );

  createSubscription(
    graphql(`
      subscription DashboardLayout_UserUsageUpdateStream($userId: ID!) {
        userUsageUpdateStream(userId: $userId) {
          id

          usage {
            totalCharacterCount
            totalBlobSize
          }
        }
      }
    `),
    () => ({ userId }),
  );

  const paneGroup = setupPaneGroup(siteId, {
    userId,
    navigate: (path, opts) => goto(path, opts),
    onSiteChange: (id) => {
      if (query.data.me.sites.some((site) => site.id === id)) {
        app.preference.current.currentSiteId = id;
      }
    },
  });

  setupEditorRegistry();

  $effect(() => {
    if (!app.preference.current.currentSiteId) {
      untrack(() => (app.preference.current.currentSiteId = currentSite.id));
    }
  });

  $effect(() => {
    if (app.state.nextCurrentSiteId && query.data.me.sites.some((s) => s.id === app.state.nextCurrentSiteId)) {
      paneGroup.switchToSite(app.state.nextCurrentSiteId);
      app.state.nextCurrentSiteId = undefined;
    }
  });

  // currentSiteId가 유효하지 않으면 (사이트 삭제 등) 첫 번째 사이트로 전환
  $effect(() => {
    const sites = query.data.me.sites;
    if (sites.length > 0 && !sites.some((s) => s.id === app.preference.current.currentSiteId)) {
      paneGroup.switchToSite(sites[0].id);
    }
  });

  $effect(() => {
    app.state.usage.current.totalCharacterCount = query.data.me.usage.totalCharacterCount;
    app.state.usage.current.totalBlobSize = query.data.me.usage.totalBlobSize;

    app.state.usage.limit.totalCharacterCount =
      query.data.me.subscription?.plan.rule.maxTotalCharacterCount ?? defaultPlanRules.maxTotalCharacterCount;
    app.state.usage.limit.totalBlobSize = String(
      query.data.me.subscription?.plan.rule.maxTotalBlobSize ?? defaultPlanRules.maxTotalBlobSize,
    );
  });

  let referralWelcomeModalOpen = $state(false);
  let marketingConsentModalOpen = $state(false);
  let userSurveyModalOpen = $state(false);
  let trialExpiredModalOpen = $state(false);

  const textReplacementRulesJson = $derived.by(() =>
    stringify(
      query.data.me.textReplacements
        .map((item) => {
          if (item.__typename === 'TextReplacementPreference') {
            if (item.state !== 'ACTIVE') return null;
            return {
              id: item.textReplacement.id,
              matchPattern: item.textReplacement.match,
              substitute: item.textReplacement.substitute,
              regex: item.textReplacement.regex,
            };
          }
          return {
            id: item.id,
            matchPattern: item.match,
            substitute: item.substitute,
            regex: item.regex,
          };
        })
        .filter((rule): rule is NonNullable<typeof rule> => rule !== null),
    ),
  );

  $effect(() => {
    const rules = textReplacementRulesJson;
    initWasm().then((wasm) => {
      wasm.setTextReplacementRules(JSON.parse(rules));
    });
  });

  $effect(() => {
    const enabled = app.preference.current.autoSurroundEnabled;
    initWasm().then((wasm) => {
      wasm.setAutoSurroundEnabled(enabled);
    });
  });

  $effect(() => {
    if (query.data.me) {
      mixpanel.identify(query.data.me.id);

      mixpanel.people.set({
        $email: query.data.me.email,
        $name: query.data.me.name,
        $avatar: qs.stringifyUrl({ url: query.data.me.avatar.url, query: { s: 256, f: 'png' } }),
      });
    }
  });

  $effect(() => {
    if (updated.current) {
      Updater.show({
        onRefresh: () => {
          mixpanel.track('reload_app', { reason: 'update' });
          location.reload();
        },
      });
    }
  });

  onMount(pollBootstrapAssertion);

  onMount(() => {
    if (query.data.me.referral && !app.preference.current.referralWelcomeModalShown) {
      referralWelcomeModalOpen = true;
      app.preference.current.referralWelcomeModalShown = true;
    } else if (query.data.me.surveys.includes('trial_expired_modal')) {
      trialExpiredModalOpen = true;
    } else if (query.data.me.marketingConsentAskedAt === null && query.data.me.usage.totalCharacterCount >= 100) {
      marketingConsentModalOpen = true;
    }

    const skipUntil = localStorage.getItem('surveySkipUntil');
    const shouldShowSurvey = query.data.me.surveys.includes('202509_ir') && (!skipUntil || new Date(skipUntil) < new Date());

    if (shouldShowSurvey && !marketingConsentModalOpen && !trialExpiredModalOpen) {
      userSurveyModalOpen = true;
    }

    if (query.data.me.preferences.initialPage) {
      app.preference.current.initialPage = query.data.me.preferences.initialPage;
    }

    if (query.data.me.preferences.toolbarStyle) {
      app.preference.current.toolbarStyle = query.data.me.preferences.toolbarStyle;
    }
  });
</script>

{#if isMobileDevice()}
  <div
    style:--grid-line-color={token('colors.decoration.grid.brand')}
    style:--cross-line-color={token('colors.decoration.grid.brand.subtle')}
    style:--grid-size="30px"
    style:--line-thickness="1px"
    class={center({
      width: '[100dvw]',
      height: '[100dvh]',
      overflowY: 'auto',
      backgroundColor: 'surface.default',
      backgroundImage:
        '[repeating-linear-gradient(0deg, transparent, transparent calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) var(--grid-size)), repeating-linear-gradient(90deg, transparent, transparent calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) var(--grid-size)), repeating-linear-gradient(0deg, transparent, transparent calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2), transparent calc(var(--grid-size) / 2), transparent var(--grid-size)), repeating-linear-gradient(90deg, transparent, transparent calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2), transparent calc(var(--grid-size) / 2), transparent var(--grid-size))]',
      backgroundSize: 'var(--grid-size) var(--grid-size)',
    })}
  >
    <div
      class={flex({
        flexDirection: 'column',
        gap: '24px',
        borderRadius: '12px',
        margin: '20px',
        padding: { base: '24px', lg: '48px' },
        width: 'full',
        maxWidth: '400px',
        backgroundColor: 'surface.default',
        boxShadow: 'medium',
      })}
    >
      <div class={flex({ justifyContent: 'flex-start' })}>
        <Logo class={css({ height: '32px' })} />
      </div>

      <div class={flex({ flexDirection: 'column', gap: '4px', wordBreak: 'keep-all' })}>
        <h1 class={css({ fontSize: { base: '22px', lg: '24px' }, fontWeight: 'extrabold' })}>
          타이피 앱에서
          <br />
          글쓰기를 이어가 보세요
        </h1>

        <div class={css({ fontSize: { base: '13px', lg: '14px' }, color: 'text.faint' })}>
          모바일에서도 타이피의 몰입감 있는 글쓰기 환경을 그대로 이용하실 수 있어요.
        </div>
      </div>

      <div class={css({ borderRadius: '8px', paddingY: '8px', textAlign: 'center', backgroundColor: 'surface.subtle' })}>
        <p class={css({ fontSize: '13px', color: 'text.faint' })}>현재 로그인 정보</p>
        <p class={css({ marginTop: '2px', fontSize: '14px' })}>{query.data.me.email}</p>
      </div>

      <HorizontalDivider color="secondary" />

      <div class={flex({ flexDirection: 'column', gap: '8px' })}>
        <Button style={css.raw({ width: 'full' })} external href="/app" size="lg" type="link" variant="primary">타이피 앱 바로가기</Button>

        <Button
          style={css.raw({ width: 'full' })}
          onclick={() => {
            mixpanel.track('logout', { via: 'mobile_dashboard' });

            location.href = qs.stringifyUrl({
              url: '/logout',
              query: {
                redirect_uri: env.PUBLIC_WEBSITE_URL,
              },
            });
          }}
          size="lg"
          variant="secondary"
        >
          로그아웃
        </Button>
      </div>
    </div>
  </div>
{:else}
  <div class={flex({ flexDirection: 'column', height: '[100dvh]' })}>
    <EnvironmentBanner />
    <AdminImpersonateBanner query$key={query.data} />

    <div
      class={flex({
        position: 'relative',
        flexGrow: '1',
        alignItems: 'stretch',
        overflow: 'hidden',
      })}
    >
      <Sidebar user$key={query.data.me} />

      <div
        class={cx(
          'main-container',
          flex({
            flexGrow: '1',
            overflow: 'auto',
          }),
        )}
      >
        {@render children()}
      </div>
    </div>
  </div>
{/if}

<CommandPalette user$key={query.data.me} />
<Notes />
<PreferenceModal user$key={query.data.me} />
<SiteSettingsModal site$key={currentSite} user$key={query.data.me} />
<DocumentExportModal user$key={query.data.me} />
<ShareModal />
<StatsModal />
<TrashModal siteId={currentSite.id} />
<Shortcuts query$key={query.data} />
<ShortcutsModal />

<PlanUpgradeModal user$key={query.data.me} />

<ReferralWelcomeModal bind:open={referralWelcomeModalOpen} />
<MarketingConsentModal bind:open={marketingConsentModalOpen} />
<TrialExpiredModal user$key={query.data.me} bind:open={trialExpiredModalOpen} />
<UserSurveyModal bind:open={userSurveyModalOpen} />

<div
  class={cx(
    'tooltip-container',
    css({
      position: 'fixed',
      inset: '0',
      zIndex: 'tooltip',
      pointerEvents: 'none',
      overflow: 'hidden',
    }),
  )}
></div>
