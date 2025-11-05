<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import dayjs from 'dayjs';
  import { disassemble } from 'es-hangul';
  import { matchSorter } from 'match-sorter';
  import mixpanel from 'mixpanel-browser';
  import * as R from 'remeda';
  import { tick } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import { match } from 'ts-pattern';
  import { PostType } from '@/enums';
  import ArrowDownIcon from '~icons/lucide/arrow-down';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import CornerDownLeftIcon from '~icons/lucide/corner-down-left';
  import FileIcon from '~icons/lucide/file';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import LineSquiggleIcon from '~icons/lucide/line-squiggle';
  import Maximize2Icon from '~icons/lucide/maximize-2';
  import SearchIcon from '~icons/lucide/search';
  import SettingsIcon from '~icons/lucide/settings';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import XIcon from '~icons/lucide/x';
  import { beforeNavigate, goto, pushState } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import type { Component } from 'svelte';
  import type { DashboardLayout_CommandPalette_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_CommandPalette_user;
  };

  type Command = {
    name: string;
    aliases: string[];
    icon: Component;
    action: () => void | Promise<void>;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_CommandPalette_user on User {
        id
        name

        sites {
          id
        }

        recentlyViewedEntities {
          id
          slug

          node {
            __typename

            ... on Post {
              id
              title
              type
            }

            ... on Canvas {
              id
              title
            }
          }
        }
      }
    `),
  );

  const searchQuery = graphql(`
    query DashboardLayout_CommandPalette_Search_Query($siteId: ID!, $query: String!) @client {
      search(siteId: $siteId, query: $query) {
        totalHits

        hits {
          __typename

          ... on SearchHitPost {
            title
            subtitle
            text

            post {
              id
              title
              type

              entity {
                id
                slug
              }
            }
          }

          ... on SearchHitCanvas {
            title

            canvas {
              id
              title

              entity {
                id
                slug
              }
            }
          }
        }
      }
    }
  `);

  const createPost = graphql(`
    mutation DashboardLayout_CommandPalette_CreatePost_Mutation($input: CreatePostInput!) {
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

  const commands: Command[] = $derived([
    {
      name: '새 포스트 만들기',
      aliases: ['새 포스트 생성', '새 글 쓰기', '새 글 생성'],
      icon: SquarePenIcon,
      action: async () => {
        const resp = await createPost({
          siteId: $user.sites[0].id,
        });

        mixpanel.track('create_post', { via: 'command_palette' });

        await goto(`/${resp.entity.slug}`);
      },
    },
    {
      name: app.preference.current.zenModeEnabled ? '집중 모드 끄기' : '집중 모드 켜기',
      aliases: ['zen mode'],
      icon: Maximize2Icon,
      action: () => {
        app.preference.current.zenModeEnabled = !app.preference.current.zenModeEnabled;
        if (app.preference.current.zenModeEnabled) {
          mixpanel.track('zen_mode_enabled', { via: 'command_palette' });
        } else {
          mixpanel.track('zen_mode_disabled', { via: 'command_palette' });
        }
      },
    },
    {
      name: '설정 열기',
      aliases: [],
      icon: SettingsIcon,
      action: () => {
        pushState('', { shallowRoute: '/preference/profile' });
      },
    },
  ]);

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

  let greeting = $state('');

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

    const randomIndex = Math.floor(Math.random() * currentGreetings.length);
    greeting = currentGreetings[randomIndex];
  };

  const debouncedSearch = R.funnel(
    async (query: string) => {
      await searchQuery.load({ siteId: $user.sites[0].id, query });
      if (selectedResultIndex !== -1) {
        selectedResultIndex = null;
      }
    },
    {
      reducer: (_, query: string) => query,
      minQuietPeriodMs: 16,
      triggerAt: 'end',
    },
  );

  let inputEl = $state<HTMLInputElement>();
  let listEl = $state<HTMLDivElement>();

  let query = $state('');
  let selectedResultIndex = $state<number | null>(null);

  const commandHits = $derived(
    matchSorter(commands, disassemble(query), {
      keys: [(item) => disassemble(item.name), (item) => item.aliases.map((v) => disassemble(v))],
      sorter: (items) => items,
    }).map((item) => ({
      __typename: 'SearchHitCommand' as const,
      ...item,
    })),
  );

  const recentlyViewedHits = $derived(
    query.length === 0
      ? $user.recentlyViewedEntities
          .slice(0, 5)
          .map((entity) =>
            match(entity.node)
              .with({ __typename: 'Post' }, (node) => ({
                __typename: 'SearchHitRecent' as const,
                entity,
                title: node.title || '제목 없음',
                icon: node.type === PostType.TEMPLATE ? LayoutTemplateIcon : FileIcon,
              }))
              .with({ __typename: 'Canvas' }, (node) => ({
                __typename: 'SearchHitRecent' as const,
                entity,
                title: node.title || '제목 없음',
                icon: LineSquiggleIcon,
              }))
              .with({ __typename: 'Folder' }, () => null)
              .exhaustive(),
          )
          .filter((hit): hit is NonNullable<typeof hit> => hit !== null)
      : [],
  );

  const searchHits = $derived(
    [...(query.length > 0 ? ($searchQuery?.search.hits ?? []) : recentlyViewedHits), ...commandHits].map((hit, idx) => ({
      ...hit,
      idx,
      action: match(hit)
        .with({ __typename: 'SearchHitCommand' }, (hit) => hit.action)
        .with({ __typename: 'SearchHitPost' }, (hit) => () => goto(`/${hit.post.entity.slug}`))
        .with({ __typename: 'SearchHitCanvas' }, (hit) => () => goto(`/${hit.canvas.entity.slug}`))
        .with({ __typename: 'SearchHitRecent' }, (hit) => () => goto(`/${hit.entity.slug}`))
        .exhaustive(),
    })),
  );
  const searchHitsByType = $derived.by(() => {
    type SearchHit = (typeof searchHits)[number];

    const map = new SvelteMap<SearchHit['__typename'], SearchHit[]>();

    for (const [idx, hit] of searchHits.entries()) {
      const key = hit.__typename;
      map.set(key, [...(map.get(key) ?? []), { ...hit, idx }]);
    }

    return map;
  });

  const handleKeyDown = async (event: KeyboardEvent) => {
    const metaOrCtrlKeyOnly = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey);
    if (metaOrCtrlKeyOnly && event.key === 'k') {
      event.preventDefault();
      app.state.commandPaletteOpen = !app.state.commandPaletteOpen;
    } else if (app.state.commandPaletteOpen) {
      if (event.key === 'Escape') {
        event.stopPropagation();
        close();
        return;
      }

      if (event.key === 'Enter') {
        event.preventDefault();

        if (selectedResultIndex === null) {
          selectedResultIndex = 0;
        } else {
          searchHits[selectedResultIndex].action();
          close();
        }

        return;
      }

      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();

        if (selectedResultIndex === null) {
          selectedResultIndex = 0;
        } else if (event.key === 'ArrowDown') {
          selectedResultIndex = selectedResultIndex + 1;
          if (selectedResultIndex >= searchHits.length) {
            selectedResultIndex = 0;
          }
        } else if (event.key === 'ArrowUp') {
          selectedResultIndex = selectedResultIndex - 1;
          if (selectedResultIndex < -1) {
            selectedResultIndex = searchHits.length - 1;
          }
        }

        await tick();
        const selectedElem = listEl?.querySelector<HTMLElement>(`& > [aria-selected="true"]`);

        if (
          selectedElem &&
          listEl &&
          (selectedElem.offsetTop < listEl.scrollTop ||
            selectedElem.offsetTop + selectedElem.clientHeight > listEl.scrollTop + listEl.clientHeight)
        ) {
          selectedElem.scrollIntoView({
            block: 'nearest',
          });
        }
      }
    }
  };

  const close = () => {
    app.state.commandPaletteOpen = false;

    query = '';
    selectedResultIndex = null;
  };

  $effect(() => {
    if (query.length > 0) {
      debouncedSearch.call(query);
    }
  });

  $effect(() => {
    if (app.state.commandPaletteOpen) {
      updateGreeting();
    }
  });

  beforeNavigate(() => {
    close();
  });
</script>

<svelte:window onkeydown={handleKeyDown} />

<Modal
  style={css.raw({ maxWidth: '600px', height: '600px', backgroundColor: 'surface.subtle' })}
  onclose={close}
  open={app.state.commandPaletteOpen}
>
  <div
    class={flex({
      position: 'relative',
      alignItems: 'center',
      marginX: '12px',
      marginY: '12px',
    })}
  >
    <input
      bind:this={inputEl}
      class={css({
        width: 'full',
        paddingLeft: '40px',
        paddingRight: '80px',
        paddingY: '6px',
        fontSize: '15px',
        fontWeight: 'medium',
      })}
      aria-live={query ? 'polite' : 'off'}
      onkeydown={(e) => {
        if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && e.isComposing) {
          e.preventDefault();
          e.stopPropagation();
        }
      }}
      placeholder={`${$user.name}님, ${greeting}`}
      tabindex="0"
      type="text"
      bind:value={query}
    />

    <div class={center({ position: 'absolute', left: '8px', top: '1/2', translate: 'auto', translateY: '-1/2', pointerEvents: 'none' })}>
      <Icon style={css.raw({ color: 'text.disabled' })} icon={SearchIcon} size={18} />
    </div>

    <div
      class={center({
        position: 'absolute',
        right: '8px',
        top: '1/2',
        gap: '12px',
        translate: 'auto',
        translateY: '-1/2',
        pointerEvents: 'none',
      })}
    >
      {#if query}
        <button
          class={center({
            borderRadius: 'full',
            size: '16px',
            color: 'text.faint',
            backgroundColor: 'surface.muted',
            pointerEvents: 'auto',
          })}
          onclick={() => {
            query = '';
            selectedResultIndex = null;
            inputEl?.focus();
          }}
          type="button"
        >
          <Icon icon={XIcon} size={12} />
        </button>
      {/if}

      <kbd
        class={center({
          gap: '2px',
          borderRadius: '4px',
          paddingX: '6px',
          paddingY: '2px',
          fontFamily: 'mono',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.faint',
          backgroundColor: 'surface.muted',
        })}
      >
        <span>{navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}</span>
        {#if !navigator.platform.includes('Mac')}
          <span>+</span>
        {/if}
        <span>K</span>
      </kbd>
    </div>
  </div>

  <div class={css({ height: '1px', backgroundColor: 'interactive.hover' })}></div>

  <div bind:this={listEl} class={flex({ flexDirection: 'column', flexGrow: '1', paddingX: '12px', overflowY: 'auto' })}>
    {#each searchHitsByType.entries() as [type, hits], index (index)}
      <div
        class={css({
          marginTop: '12px',
          marginBottom: '4px',
          paddingX: '8px',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.faint',
        })}
      >
        {match(type)
          .with('SearchHitCommand', () => '액션')
          .with('SearchHitPost', () => '포스트')
          .with('SearchHitCanvas', () => '캔버스')
          .with('SearchHitRecent', () => '최근 본 항목')
          .exhaustive()}
      </div>

      {#each hits as hit (hit.idx)}
        <button
          class={flex({
            alignItems: 'center',
            gap: '12px',
            borderRadius: '6px',
            paddingX: '8px',
            paddingY: '6px',
            _hover: { backgroundColor: 'surface.muted' },
            _selected: { backgroundColor: 'surface.muted' },
            _focus: { backgroundColor: 'surface.muted' },
            '& em': { color: 'text.brand' },
          })}
          aria-selected={selectedResultIndex === hit.idx}
          onclick={() => {
            mixpanel.track('command_palette_select', {
              type: hit.__typename,
              ...(hit.__typename === 'SearchHitCommand' ? { name: hit.name } : {}),
            });

            hit.action();
            close();
          }}
          onfocus={() => (selectedResultIndex = hit.idx)}
          role="option"
          tabindex="0"
          type="button"
        >
          {#if hit.__typename === 'SearchHitCommand'}
            <div
              class={center({ flexShrink: '0', borderRadius: '6px', size: '24px', color: 'text.faint', backgroundColor: 'surface.muted' })}
            >
              <Icon icon={hit.icon} size={16} />
            </div>

            <span class={css({ fontSize: '14px', fontWeight: 'medium' })}>{hit.name}</span>
          {:else if hit.__typename === 'SearchHitPost'}
            <div
              class={center({ flexShrink: '0', borderRadius: '6px', size: '24px', color: 'text.faint', backgroundColor: 'surface.muted' })}
            >
              {#if hit.post.type === PostType.TEMPLATE}
                <Icon icon={LayoutTemplateIcon} size={16} />
              {:else}
                <Icon icon={FileIcon} size={16} />
              {/if}
            </div>

            <div class={css({ fontSize: '14px', fontWeight: 'medium', truncate: true })}>
              {#if hit.title}
                <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                {@html hit.title}
              {:else}
                {hit.post.title}
              {/if}
            </div>

            {#if hit.text}
              <div class={css({ color: 'text.muted', fontSize: '12px', truncate: true })}>
                <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                {@html hit.text}
              </div>
            {/if}
          {:else if hit.__typename === 'SearchHitCanvas'}
            <div
              class={center({ flexShrink: '0', borderRadius: '6px', size: '24px', color: 'text.faint', backgroundColor: 'surface.muted' })}
            >
              <Icon icon={LineSquiggleIcon} size={16} />
            </div>

            <div class={css({ fontSize: '14px', fontWeight: 'medium', truncate: true })}>
              {#if hit.title}
                <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                {@html hit.title}
              {:else}
                {hit.canvas.title || '제목 없음'}
              {/if}
            </div>
          {:else if hit.__typename === 'SearchHitRecent'}
            <div
              class={center({ flexShrink: '0', borderRadius: '6px', size: '24px', color: 'text.faint', backgroundColor: 'surface.muted' })}
            >
              <Icon icon={hit.icon} size={16} />
            </div>

            <span class={css({ fontSize: '14px', fontWeight: 'medium', truncate: true })}>{hit.title}</span>
          {/if}
        </button>
      {/each}
    {:else}
      <div
        class={center({
          flexDirection: 'column',
          flexGrow: '1',
          width: 'full',
          color: 'text.muted',
          gap: '2px',
        })}
      >
        <div class={css({ fontSize: '16px', fontWeight: 'medium' })}>검색 결과가 없습니다</div>
        <div class={css({ fontSize: '14px' })}>다른 검색어를 입력해보세요</div>
      </div>
    {/each}
  </div>

  <div class={css({ height: '1px', backgroundColor: 'interactive.hover' })}></div>

  <div
    class={flex({
      alignItems: 'center',
      gap: '16px',
      paddingX: '12px',
      paddingY: '12px',
      color: 'text.faint',
      backgroundColor: 'surface.muted',
    })}
  >
    <div class={flex({ alignItems: 'center', gap: '8px' })}>
      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <div class={center({ flexShrink: '0', borderWidth: '1px', borderRadius: '6px', size: '22px' })}>
          <Icon icon={ArrowUpIcon} size={14} />
        </div>

        <div class={center({ flexShrink: '0', borderWidth: '1px', borderRadius: '6px', size: '22px' })}>
          <Icon icon={ArrowDownIcon} size={14} />
        </div>
      </div>

      <span class={css({ fontSize: '13px', fontWeight: 'semibold' })}>이동</span>
    </div>

    <div class={flex({ alignItems: 'center', gap: '8px' })}>
      <div class={center({ flexShrink: '0', borderWidth: '1px', borderRadius: '6px', size: '22px' })}>
        <Icon icon={CornerDownLeftIcon} size={14} />
      </div>

      <span class={css({ fontSize: '13px', fontWeight: 'semibold' })}>선택</span>
    </div>

    <div class={flex({ alignItems: 'center', gap: '8px' })}>
      <div class={center({ flexShrink: '0', borderWidth: '1px', borderRadius: '6px', paddingX: '4px', height: '22px' })}>
        <div class={css({ fontSize: '10px', fontWeight: 'bold' })}>ESC</div>
      </div>

      <span class={css({ fontSize: '13px', fontWeight: 'semibold' })}>닫기</span>
    </div>
  </div>
</Modal>
