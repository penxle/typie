<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import qs from 'query-string';
  import { untrack } from 'svelte';
  import Logo from '$assets/logos/logo.svg?component';
  import { graphql } from '$graphql';
  import { Button, HorizontalDivider } from '$lib/components';
  import { setupAppContext } from '$lib/context';
  import { isMobileDevice } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import { token } from '$styled-system/tokens';
  import ShareModal from './@share/ShareModal.svelte';
  import CommandPalette from './CommandPalette.svelte';
  import Sidebar from './Sidebar.svelte';

  let { children } = $props();

  const query = graphql(`
    query DashboardLayout_Query {
      me @required {
        id
        name
        email

        avatar {
          id
          url
        }

        sites {
          id
          name
        }

        ...DashboardLayout_Sidebar_user
        ...DashboardLayout_CommandPalette_user
      }
    }
  `);

  const siteUpdateStream = graphql(`
    subscription DashboardLayout_SiteUpdateStream($siteId: ID!) {
      siteUpdateStream(siteId: $siteId) {
        ... on Site {
          id

          ...DashboardLayout_EntityTree_site
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

  setupAppContext();

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
</script>

{#if isMobileDevice()}
  <div
    style:--grid-line-color={token('colors.brand.100')}
    style:--cross-line-color={token('colors.brand.50')}
    style:--grid-size="30px"
    style:--line-thickness="1px"
    class={center({
      width: 'screen',
      height: 'screen',
      overflowY: 'auto',
      backgroundColor: 'white',
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
        backgroundColor: 'white',
        boxShadow: 'medium',
      })}
    >
      <div class={flex({ justifyContent: 'flex-start' })}>
        <Logo class={css({ height: '20px' })} />
      </div>

      <div class={flex({ flexDirection: 'column', gap: '4px', wordBreak: 'keep-all' })}>
        <h1 class={css({ fontSize: { base: '22px', lg: '24px' }, fontWeight: 'extrabold' })}>
          아직 모바일에서는
          <br />
          서비스를 제공하지 않아요
        </h1>

        <div class={css({ fontSize: { base: '13px', lg: '14px' }, color: 'gray.500' })}>
          PC에서 접속하시면 타이피의 모든 기능을 이용하실 수 있어요.
        </div>
      </div>

      <div class={css({ borderRadius: '8px', paddingY: '8px', textAlign: 'center', backgroundColor: 'gray.50' })}>
        <p class={css({ fontSize: '13px', color: 'gray.500' })}>현재 로그인 정보</p>
        <p class={css({ marginTop: '2px', fontSize: '14px' })}>{$query.me.email}</p>
      </div>

      <HorizontalDivider color="secondary" />

      <div>
        <Button
          style={css.raw({ width: 'full' })}
          external
          href="https://x.com/intent/follow?screen_name=typieofficial"
          size="lg"
          type="link"
          variant="secondary"
        >
          타이피 트위터 팔로우하기
        </Button>

        <p class={css({ marginTop: '6px', fontSize: '11px', textAlign: 'center', color: 'gray.500' })}>
          타이피 트위터 팔로우하고 최근 소식을 가장 빠르게 받아보세요
        </p>
      </div>
    </div>
  </div>
{:else}
  <div
    class={flex({
      position: 'relative',
      alignItems: 'stretch',
      height: 'screen',
      backgroundColor: 'gray.100',
    })}
  >
    <Sidebar $user={$query.me} />

    <div
      class={css({
        flexGrow: '1',
        borderWidth: '[0.5px]',
        borderRadius: '4px',
        marginY: '8px',
        marginRight: '8px',
        backgroundColor: 'white',
        boxShadow: '[0 3px 6px -2px {colors.gray.950/3}, 0 1px 1px {colors.gray.950/5}]',
        overflowY: 'auto',
      })}
    >
      {@render children()}
    </div>
  </div>
{/if}

<CommandPalette $user={$query.me} />
<ShareModal />
