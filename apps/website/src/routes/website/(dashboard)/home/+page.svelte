<script lang="ts">
  import dayjs from 'dayjs';
  import { untrack } from 'svelte';
  import { match } from 'ts-pattern';
  import ClockIcon from '~icons/lucide/clock';
  import { graphql } from '$graphql';
  import { Helmet, Icon, Img } from '$lib/components';
  import { typewriter } from '$lib/transitions';
  import { css } from '$styled-system/css';
  import { center, flex, grid } from '$styled-system/patterns';
  import TopBar from '../TopBar.svelte';
  import ActivityGrid from './ActivityGrid.svelte';

  const query = graphql(`
    query HomePage_Query {
      me @required {
        id
        name

        recentPosts {
          id
          title
          subtitle
          excerpt
          updatedAt

          coverImage {
            id
            ...Img_image
          }

          entity {
            id
            slug
          }
        }

        ...HomePage_ActivityGrid_user
      }
    }
  `);

  const greetings = {
    morning: ['상쾌한 아침이에요.', '오늘 하루도 시작해볼까요?', '활기찬 아침이에요.', '좋은 아침이에요.', '새로운 하루가 시작됐어요.'],
    afternoon: [
      '점심은 맛있게 드셨나요?',
      '오후도 즐겁게 보내세요.',
      '활기찬 오후예요.',
      '커피 한 잔과 함께하는 시간 되세요.',
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

  let lastGreetingIdx = $state<number>();
  let greeting = $state<string>();

  let timer = $state<NodeJS.Timeout>();

  const updateGreeting = () => {
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

    while (true) {
      const randomIndex = Math.floor(Math.random() * currentGreetings.length);
      if (randomIndex !== lastGreetingIdx) {
        lastGreetingIdx = randomIndex;
        greeting = currentGreetings[randomIndex];
        break;
      }
    }
  };

  $effect(() => {
    untrack(() => {
      updateGreeting();
    });
  });

  $effect(() => {
    return () => {
      if (timer) {
        clearTimeout(timer);
      }
    };
  });
</script>

<Helmet title="홈" />

<TopBar />

<div class={center({ flexDirection: 'column', flexGrow: '1', width: 'full' })}>
  <div class={flex({ flexDirection: 'column', flexGrow: '1', gap: '100px', marginY: '64px', width: 'full', maxWidth: '1000px' })}>
    <div class={center({ gap: '4px', fontSize: '32px', fontWeight: 'bold', letterSpacing: '[0]' })}>
      <span>
        {$query.me.name}님,
      </span>

      {#if greeting}
        <span
          onintroend={() => {
            if (timer) {
              clearTimeout(timer);
            }

            timer = setTimeout(() => {
              greeting = undefined;
            }, 60 * 1000);
          }}
          onoutroend={() => {
            updateGreeting();
          }}
          in:typewriter={{ speed: 25 }}
          out:typewriter={{ speed: 25 }}
        >
          {greeting}
        </span>
      {/if}
    </div>

    <div class={flex({ flexDirection: 'column', gap: '16px' })}>
      <div class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'gray.500' })}>최근 편집 포스트</div>

      <div class={grid({ gridTemplateColumns: 'repeat(5, minmax(0, 1fr))', gap: '16px' })}>
        {#each $query.me.recentPosts as post (post.id)}
          <a
            class={flex({
              flexDirection: 'column',
              borderWidth: '1px',
              borderRadius: '12px',
              minHeight: '200px',
              overflow: 'hidden',
              userSelect: 'none',
            })}
            href={`/${post.entity.slug}`}
          >
            {#if post.coverImage}
              <Img $image={post.coverImage} alt={`${post.title} 커버 이미지`} ratio={5 / 2} size="full" />
            {:else}
              <div class={css({ width: 'full', aspectRatio: '[5/2]', backgroundColor: 'gray.100' })}></div>
            {/if}

            <div class={flex({ flexDirection: 'column', flexGrow: '1', paddingX: '12px', paddingY: '8px' })}>
              <div class={css({ fontSize: '14px', fontWeight: 'semibold', lineClamp: '1' })}>{post.title}</div>
              {#if post.subtitle}
                <div class={css({ marginTop: '2px', fontSize: '12px', fontWeight: 'medium', color: 'gray.500', lineClamp: '1' })}>
                  {post.subtitle}
                </div>
              {/if}

              <div class={css({ marginTop: '8px', fontSize: '12px', color: 'gray.500', lineClamp: '2' })}>{post.excerpt}</div>

              <div class={css({ flexGrow: '1' })}></div>

              <div class={flex({ alignItems: 'center', gap: '4px', marginTop: '12px' })}>
                <Icon style={css.raw({ color: 'gray.400' })} icon={ClockIcon} size={14} />
                <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'gray.500' })}>{dayjs(post.updatedAt).fromNow()}</span>
              </div>
            </div>
          </a>
        {/each}
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '16px' })}>
      <div class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'gray.500' })}>나의 기록</div>

      <ActivityGrid $user={$query.me} />
    </div>
  </div>
</div>
