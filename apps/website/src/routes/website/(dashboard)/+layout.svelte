<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { Button, HorizontalDivider } from '@typie/ui/components';
  import { setupAppContext } from '@typie/ui/context';
  import { isMobileDevice } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import qs from 'query-string';
  import { onMount, untrack } from 'svelte';
  import Logo from '$assets/logos/logo.svg?component';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { AdminImpersonateBanner } from '$lib/components/admin';
  import { setupSplitViewContext } from './[slug]/@split-view/context.svelte';
  import { setupDragDropContext } from './[slug]/@split-view/drag-context.svelte';
  import Notes from './@notes/Notes.svelte';
  import ShareModal from './@share/ShareModal.svelte';
  import CanvasDeprecationModal from './CanvasDeprecationModal.svelte';
  import CommandPalette from './CommandPalette.svelte';
  import ReferralWelcomeModal from './ReferralWelcomeModal.svelte';
  import Shortcuts from './Shortcuts.svelte';
  import Sidebar from './Sidebar.svelte';
  import UserSurveyModal from './UserSurveyModal.svelte';

  let { children } = $props();

  const query = graphql(`
    query DashboardLayout_Query {
      me @required {
        id
        name
        email
        preferences

        avatar {
          id
          url
        }

        sites {
          id
          name
        }

        referral {
          id
        }

        surveys

        ...DashboardLayout_Sidebar_user
        ...DashboardLayout_CommandPalette_user
      }

      ...AdminImpersonateBanner_query
      ...DashboardLayout_Shortcuts_query
      ...DashboardLayout_Notes_query
    }
  `);

  const siteUpdateStream = graphql(`
    subscription DashboardLayout_SiteUpdateStream($siteId: ID!) {
      siteUpdateStream(siteId: $siteId) {
        ... on Site {
          id

          ...DashboardLayout_EntityTree_site
          ...DashboardLayout_Trash_site
        }

        ... on Entity {
          id
          state

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

            ... on Canvas {
              id
              title
            }
          }
        }
      }
    }
  `);

  const siteUsageUpdateStream = graphql(`
    subscription DashboardLayout_SiteUsageUpdateStream($siteId: ID!) {
      siteUsageUpdateStream(siteId: $siteId) {
        ... on Site {
          id

          usage {
            totalCharacterCount
            totalBlobSize
          }
        }
      }
    }
  `);

  const app = setupAppContext($query.me.id);

  setupSplitViewContext($query.me.id);
  setupDragDropContext();

  let referralWelcomeModalOpen = $state(false);
  let userSurveyModalOpen = $state(false);
  let canvasDeprecationModalOpen = $state(false);

  $effect(() => {
    return untrack(() => {
      const unsubscribe = siteUpdateStream.subscribe({ siteId: $query.me.sites[0].id });
      const unsubscribe2 = siteUsageUpdateStream.subscribe({ siteId: $query.me.sites[0].id });

      return () => {
        unsubscribe();
        unsubscribe2();
      };
    });
  });

  $effect(() => {
    if ($query.me) {
      mixpanel.identify($query.me.id);

      mixpanel.people.set({
        $email: $query.me.email,
        $name: $query.me.name,
        $avatar: qs.stringifyUrl({ url: $query.me.avatar.url, query: { s: 256, f: 'png' } }),
      });
    }
  });

  onMount(() => {
    if ($query.me.referral && !app.preference.current.referralWelcomeModalShown) {
      referralWelcomeModalOpen = true;
      app.preference.current.referralWelcomeModalShown = true;
    }

    const skipUntil = localStorage.getItem('surveySkipUntil');
    const shouldShowSurvey = $query.me.surveys.includes('202509_ir') && (!skipUntil || new Date(skipUntil) < new Date());

    if (shouldShowSurvey) {
      userSurveyModalOpen = true;
    }

    const canvasDeprecationSkipUntil = localStorage.getItem('canvasDeprecationSkipUntil');
    const shouldShowCanvasDeprecation = !canvasDeprecationSkipUntil || new Date(canvasDeprecationSkipUntil) < new Date();

    if (shouldShowCanvasDeprecation) {
      canvasDeprecationModalOpen = true;
    }

    if ($query.me.preferences.initialPage) {
      app.preference.current.initialPage = $query.me.preferences.initialPage;
    }

    if ($query.me.preferences.toolbarStyle) {
      app.preference.current.toolbarStyle = $query.me.preferences.toolbarStyle;
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
        <p class={css({ marginTop: '2px', fontSize: '14px' })}>{$query.me.email}</p>
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
    <AdminImpersonateBanner {$query} />

    <div
      class={flex({
        position: 'relative',
        flexGrow: '1',
        alignItems: 'stretch',
        backgroundColor: 'surface.muted',
        overflow: 'hidden',
      })}
    >
      <Sidebar $user={$query.me} />

      <div
        class={cx(
          'main-container',
          flex({
            flexGrow: '1',
            marginY: '8px',
            marginRight: '8px',
            overflow: 'auto',
          }),
        )}
      >
        {@render children()}
      </div>
    </div>
  </div>
{/if}

<CommandPalette $user={$query.me} />
<Notes {$query} />
<ShareModal />
<Shortcuts {$query} />

<ReferralWelcomeModal bind:open={referralWelcomeModalOpen} />
<UserSurveyModal bind:open={userSurveyModalOpen} />
<CanvasDeprecationModal bind:open={canvasDeprecationModalOpen} />

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
