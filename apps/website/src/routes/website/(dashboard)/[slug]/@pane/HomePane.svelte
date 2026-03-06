<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, Helmet, Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { typewriter } from '@typie/ui/transitions';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { match } from 'ts-pattern';
  import { DocumentType } from '@/enums';
  import FileIcon from '~icons/lucide/file';
  import FilePenIcon from '~icons/lucide/file-pen';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import XIcon from '~icons/lucide/x';
  import { goto } from '$app/navigation';
  import Logo from '$assets/logos/logo.svg?component';
  import { graphql } from '$mearie';
  import ActivityGrid from '../../@stats/ActivityGrid.svelte';
  import CloseButton from './CloseButton.svelte';
  import { getPaneGroup, setupPane } from './context.svelte';
  import PaneSkeleton from './PaneSkeleton.svelte';
  import type { Pane } from './types';

  type HomePane = Extract<Pane, { kind: 'home' }>;

  type Props = {
    pane: HomePane;
  };

  let { pane }: Props = $props();

  const query = createQuery(
    graphql(`
      query HomePane_Query {
        me @required {
          id
          name

          ...DashboardLayout_Stats_ActivityGrid_user

          sites {
            id

            firstEntity(type: DOCUMENT) {
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

              ... on Document {
                id
                title
                documentType: type
                excerpt
              }
            }
          }
        }
      }
    `),
  );

  const [createDocument] = createMutation(
    graphql(`
      mutation HomePane_CreateDocument_Mutation($input: CreateDocumentInput!) {
        createDocument(input: $input) {
          id

          entity {
            id
            slug

            container {
              ... on Site {
                id

                entities {
                  id

                  node {
                    __typename
                  }

                  ...DashboardLayout_EntityTree_Entity_entity
                }
              }

              ... on Entity {
                id

                children {
                  id

                  node {
                    __typename
                  }

                  ...DashboardLayout_EntityTree_Entity_entity
                }
              }
            }
          }
        }
      }
    `),
  );

  const app = getAppContext();
  const paneGroup = getPaneGroup();

  const focused = $derived(pane.id === paneGroup.state.current.focusedPaneId);

  $effect(() => {
    if (focused) {
      app.state.current = undefined;
      app.state.ancestors = [];
    }
  });

  let mounted = $state(false);

  onMount(() => {
    mounted = true;
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

  setupPane(pane);
</script>

<div
  class={flex({
    position: 'relative',
    size: 'full',
    backgroundColor: 'surface.default',
    overflow: 'hidden',
  })}
  data-pane-id={pane.id}
  onclick={() => {
    paneGroup.state.current.focusedPaneId = pane.id;
  }}
  onfocusin={() => {
    paneGroup.state.current.focusedPaneId = pane.id;
  }}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      paneGroup.state.current.focusedPaneId = pane.id;
    }
  }}
  role="tabpanel"
  tabindex={0}
>
  {#if query.data}
    {#if focused}
      <Helmet title="홈" />
    {/if}

    <div class={css({ width: 'full', height: 'full', overflowY: 'auto' })}>
      <div
        class={flex({
          flexDirection: 'column',
          justifyContent: 'center',
          gap: '32px',
          width: '800px',
          maxWidth: 'full',
          minHeight: 'full',
          marginX: 'auto',
          padding: '64px',
        })}
      >
        <div class={flex({ flexDirection: 'column', gap: '12px' })}>
          <Logo class={css({ size: '32px' })} />
          {#if mounted}
            <h1
              class={css({
                fontSize: '24px',
                fontWeight: 'bold',
                color: 'text.default',
                minHeight: '36px',
                width: '800px',
                maxWidth: 'full',
              })}
              transition:typewriter={{ speed: 50 }}
            >
              {query.data.me.name}님, {getGreeting()}
            </h1>
          {/if}
        </div>

        {#if query.data.me.sites[0].firstEntity}
          {#if query.data.me.recentlyViewedEntities.length > 0}
            <div class={flex({ flexDirection: 'column', gap: '16px', width: '800px', maxWidth: 'full' })}>
              <h2 class={css({ fontSize: '18px', fontWeight: 'semibold', color: 'text.default' })}>최근 본 항목</h2>
              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                {#each query.data.me.recentlyViewedEntities.slice(0, 5) as entity (entity.id)}
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
                          icon={entity.node.__typename === 'Document' && entity.node.documentType === DocumentType.TEMPLATE
                            ? LayoutTemplateIcon
                            : FileIcon}
                        />
                        <div class={css({ fontSize: '14px', color: 'text.default', fontWeight: 'medium' })}>
                          {entity.node.__typename === 'Document' ? entity.node.title : ''}
                        </div>
                      </div>
                      {#if entity.node.__typename === 'Document' && entity.node.excerpt}
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
        {:else}
          <div
            class={center({
              width: 'full',
              flexDirection: 'column',
              gap: '20px',
              paddingY: '50px',
              borderRadius: '8px',
              borderWidth: '1px',
              textAlign: 'center',
            })}
          >
            <Icon style={css.raw({ size: '56px', color: 'text.subtle', '& *': { strokeWidth: '[1.25px]' } })} icon={FilePenIcon} />

            <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
              <h1 class={css({ fontSize: '16px', fontWeight: 'bold', color: 'text.subtle' })}>첫 문서를 만들어보세요</h1>
              <p class={css({ fontSize: '14px', color: 'text.faint' })}>아래 버튼을 눌러 문서를 만들 수 있어요</p>
            </div>

            <Button
              onclick={async () => {
                if (!query.data) return;

                const resp = await createDocument({
                  input: {
                    siteId: query.data.me.sites[0].id,
                  },
                });

                mixpanel.track('create_document', { via: 'empty_home' });

                await goto(`/${resp.createDocument.entity.slug}`);
              }}
            >
              새 문서 만들기
            </Button>
          </div>
        {/if}

        <div class={flex({ flexDirection: 'column', gap: '16px', width: 'full' })}>
          <h2 class={css({ fontSize: '18px', fontWeight: 'semibold', color: 'text.default' })}>최근 활동</h2>
          <ActivityGrid user$key={query.data.me} />
        </div>
      </div>
    </div>
  {/if}

  {#if !query.data}
    <div
      class={css({
        position: 'absolute',
        inset: '0',
        backgroundColor: 'surface.default',
      })}
      out:fade={{ duration: 150 }}
    >
      <PaneSkeleton {pane} />
    </div>
  {/if}

  {#if paneGroup.enabled && !app.preference.current.zenModeEnabled}
    <CloseButton style={css.raw({ position: 'absolute', top: '6px', right: '8px', zIndex: '1' })}>
      <Icon icon={XIcon} size={16} />
    </CloseButton>
  {/if}
</div>
