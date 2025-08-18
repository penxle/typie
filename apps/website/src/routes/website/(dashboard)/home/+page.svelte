<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, Helmet, Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { typewriter } from '@typie/ui/transitions';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { onMount } from 'svelte';
  import { match } from 'ts-pattern';
  import FileIcon from '~icons/lucide/file';
  import FilePenIcon from '~icons/lucide/file-pen';
  import LineSquiggleIcon from '~icons/lucide/line-squiggle';
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';
  import ActivityGrid from '../@stats/ActivityGrid.svelte';

  const query = graphql(`
    query HomePage_Query {
      me @required {
        id
        name

        ...DashboardLayout_Stats_ActivityGrid_user

        sites {
          id

          firstEntity(type: POST) {
            id
            slug
          }
        }

        recentlyViewedEntities {
          id
          slug
          type

          node {
            __typename

            ... on Post {
              id
              title
              subtitle
              excerpt
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

  const createPost = graphql(`
    mutation HomePage_CreatePost_Mutation($input: CreatePostInput!) {
      createPost(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const app = getAppContext();

  let mounted = $state(false);

  onMount(() => {
    mounted = true;
    app.state.current = undefined;
    app.state.ancestors = [];
  });

  const greetings = {
    morning: ['상쾌한 아침이에요.', '오늘 하루도 시작해볼까요?', '활기찬 아침이에요.', '좋은 아침이에요.', '새로운 하루가 시작됐어요.'],
    afternoon: [
      '점심은 맛있게 드셨나요?',
      '오후도 즐겁게 보내세요.',
      '활기찬 오후예요.',
      '커피 한 잔과 함께해요.',
      '오후도 즐겁게 보내고 계신가요?',
    ],
    evening: [
      '하루 어떻게 보내셨나요?',
      '저녁 식사는 하셨나요?',
      '오늘 하루 수고하셨어요.',
      '편안한 저녁이에요.',
      '저녁 시간 잘 보내고 계신가요?',
    ],
    night: ['늦은 시간까지 수고 많으세요.', '고요한 밤이에요.', '편안한 밤 시간 되세요.', '좋은 밤 되세요.', '별이 빛나는 밤이에요.'],
  };

  const getGreeting = () => {
    const currentHour = dayjs().hour();
    const currentGreetings = match(currentHour)
      .when(
        (hour) => hour >= 5 && hour < 12,
        () => greetings.morning,
      )
      .when(
        (hour) => hour >= 12 && hour < 18,
        () => greetings.afternoon,
      )
      .when(
        (hour) => hour >= 18 && hour < 23,
        () => greetings.evening,
      )
      .otherwise(() => greetings.night);

    const randomIndex = Math.floor(Math.random() * currentGreetings.length);
    return currentGreetings[randomIndex];
  };
</script>

<Helmet title="홈" />

{#if $query.me.sites[0].firstEntity}
  <div class={center({ flexDirection: 'column', gap: '32px', width: 'full', minHeight: 'full', padding: '32px' })}>
    {#if mounted}
      <h1
        class={css({ fontSize: '24px', fontWeight: 'bold', color: 'text.default', minHeight: '36px', width: '800px' })}
        transition:typewriter={{ speed: 50 }}
      >
        {$query.me.name}님, {getGreeting()}
      </h1>
    {/if}

    {#if $query.me.recentlyViewedEntities.length > 0}
      <div class={flex({ flexDirection: 'column', gap: '16px', width: '800px' })}>
        <h2 class={css({ fontSize: '18px', fontWeight: 'semibold', color: 'text.default' })}>최근 본 항목</h2>
        <div class={flex({ flexDirection: 'column', gap: '8px' })}>
          {#each $query.me.recentlyViewedEntities.slice(0, 5) as entity (entity.id)}
            <a
              class={css({
                padding: '12px',
                borderRadius: '8px',
                backgroundColor: 'surface.subtle',
                transition: 'background',
                transitionDuration: '150ms',
                _hover: {
                  backgroundColor: 'surface.muted',
                },
              })}
              href="/{entity.slug}"
            >
              <div class={flex({ flexDirection: 'column', gap: '4px' })}>
                <div class={flex({ alignItems: 'center', gap: '8px' })}>
                  <Icon
                    style={css.raw({ size: '16px', color: 'text.subtle', flexShrink: '0' })}
                    icon={entity.node.__typename === 'Canvas' ? LineSquiggleIcon : FileIcon}
                  />
                  <div class={css({ fontSize: '14px', color: 'text.default', fontWeight: 'medium' })}>
                    {entity.node.__typename === 'Post' || entity.node.__typename === 'Canvas' ? entity.node.title : ''}
                  </div>
                </div>
                {#if entity.node.__typename === 'Post' && entity.node.excerpt}
                  <div class={css({ fontSize: '13px', color: 'text.subtle', paddingLeft: '24px', lineClamp: '1' })}>
                    {entity.node.excerpt}
                  </div>
                {/if}
              </div>
            </a>
          {/each}
        </div>
      </div>
    {/if}

    <div class={flex({ flexDirection: 'column', gap: '16px', width: '800px' })}>
      <h2 class={css({ fontSize: '18px', fontWeight: 'semibold', color: 'text.default' })}>최근 활동</h2>
      <ActivityGrid $user={$query.me} />
    </div>
  </div>
{:else}
  <div class={center({ flexDirection: 'column', gap: '20px', size: 'full', textAlign: 'center' })}>
    <Icon style={css.raw({ size: '56px', color: 'text.subtle', '& *': { strokeWidth: '[1.25px]' } })} icon={FilePenIcon} />

    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
      <h1 class={css({ fontSize: '16px', fontWeight: 'bold', color: 'text.subtle' })}>첫 포스트를 만들어보세요</h1>
      <p class={css({ fontSize: '14px', color: 'text.faint' })}>아래 버튼을 눌러 포스트를 만들 수 있어요</p>
    </div>

    <Button
      onclick={async () => {
        const resp = await createPost({
          siteId: $query.me.sites[0].id,
        });

        mixpanel.track('create_post', { via: 'empty_home' });

        await goto(`/${resp.entity.slug}`);
      }}
    >
      새 포스트 만들기
    </Button>
  </div>
{/if}
