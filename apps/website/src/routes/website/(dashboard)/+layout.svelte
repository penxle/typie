<script lang="ts">
  import { createSubscription } from '@mearie/svelte';
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
  import { onMount } from 'svelte';
  import { updated } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { env } from '$env/dynamic/public';
  import { pollBootstrapAssertion } from '$lib/bootstrap';
  import { AdminImpersonateBanner } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';
  import { wasm } from '$lib/wasm';
  import { graphql } from '$mearie';
  import { setupSplitViewContext } from './[slug]/@split-view/context.svelte';
  import { setupDragDropContext } from './[slug]/@split-view/drag-context.svelte';
  import { setupEditorRegistry } from './[slug]/@split-view/editor-registry.svelte';
  import Notes from './@notes/Notes.svelte';
  import PreferenceModal from './@preference/PreferenceModal.svelte';
  import ShareModal from './@share/ShareModal.svelte';
  import SiteSettingsModal from './@site-settings/SiteSettingsModal.svelte';
  import StatsModal from './@stats/StatsModal.svelte';
  import TrashModal from './@trash/TrashModal.svelte';
  import CommandPalette from './CommandPalette.svelte';
  import MarketingConsentModal from './MarketingConsentModal.svelte';
  import ReferralWelcomeModal from './ReferralWelcomeModal.svelte';
  import Shortcuts from './Shortcuts.svelte';
  import Sidebar from './Sidebar.svelte';
  import TrialExpiredModal from './TrialExpiredModal.svelte';
  import UserSurveyModal from './UserSurveyModal.svelte';

  let { data, children } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  let siteId = $derived(query.data.me.sites[0].id);
  let userId = $derived(query.data.me.id);

  createSubscription(
    graphql(`
      subscription DashboardLayout_SiteUpdateStream($siteId: ID!) {
        siteUpdateStream(siteId: $siteId) {
          ... on Site {
            id

            ...DashboardLayout_EntityTree_site
            ...DashboardLayout_TrashModal_site
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

              ... on Post {
                id
                title

                characterCountChange {
                  additions
                  deletions
                }
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

  const app = setupAppContext(query.data.me.id);

  setupSplitViewContext(query.data.me.id);
  setupDragDropContext();
  setupEditorRegistry();

  let referralWelcomeModalOpen = $state(false);
  let marketingConsentModalOpen = $state(false);
  let userSurveyModalOpen = $state(false);
  let trialExpiredModalOpen = $state(false);

  const fontFaces = $derived(
    query.data.me.sites[0].fonts
      .flatMap((font) => [
        `@font-face { font-family: ${font.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
        `@font-face { font-family: ${font.family.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
      ])
      .join('\n'),
  );

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
    wasm.setTextReplacementRules(JSON.parse(textReplacementRulesJson));
  });

  $effect(() => {
    wasm.setAutoSurroundEnabled(app.preference.current.autoSurroundEnabled);
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

<svelte:head>
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html '<style type="text/css"' + `>${fontFaces}</` + 'style>'}
</svelte:head>

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
        <Button style={css.raw({ width: 'full' })} external gradient href="https://typie.link" size="lg" type="link" variant="primary">
          타이피 앱 바로가기
        </Button>

        <Button
          style={css.raw({ width: 'full' })}
          onclick={() => {
            mixpanel.track('logout', { via: 'mobile_dashboard' });

            location.href = qs.stringifyUrl({
              url: `${env.PUBLIC_AUTH_URL}/logout`,
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
    <AdminImpersonateBanner query$key={query.data} />

    <div
      class={flex({
        position: 'relative',
        flexGrow: '1',
        alignItems: 'stretch',
        overflow: 'hidden',
      })}
    >
      <Sidebar site$key={query.data.me.sites[0]} user$key={query.data.me} />

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
<Notes query$key={query.data} />
<PreferenceModal user$key={query.data.me} />
<SiteSettingsModal site$key={query.data.me.sites[0]} user$key={query.data.me} />
<ShareModal />
<StatsModal />
<TrashModal site$key={query.data.me.sites[0]} />
<Shortcuts query$key={query.data} />

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
