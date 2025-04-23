<script lang="ts">
  import { base64 } from 'rfc4648';
  import { onMount } from 'svelte';
  import * as YAwareness from 'y-protocols/awareness';
  import * as Y from 'yjs';
  import { browser } from '$app/environment';
  import Logo from '$assets/logos/logo.svg?component';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { Button, Helmet } from '$lib/components';
  import { setupAppContext } from '$lib/context';
  import { TiptapEditor, TiptapRenderer } from '$lib/tiptap';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import { token } from '$styled-system/tokens';
  import { YState } from '../(dashboard)/[slug]/state.svelte';
  import Toolbar from '../(dashboard)/[slug]/Toolbar.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  const query = graphql(`
    query IndexPage_Query {
      me {
        id
      }

      welcome {
        body
        update
      }
    }
  `);

  const app = setupAppContext();
  app.can.hideToolbar = false;

  let editor = $state<Ref<Editor>>();

  const doc = new Y.Doc();
  const awareness = new YAwareness.Awareness(doc);

  const maxWidth = new YState<number>(doc, 'maxWidth', 800);

  onMount(() => {
    Y.applyUpdateV2(doc, base64.parse($query.welcome.update), 'remote');
  });
</script>

<Helmet
  description="창작자가 기다려온 글쓰기 앱 타이피를 만나보세요. 기본적인 텍스트 편집은 물론, 다양한 꾸밈 요소와 글쓰기 편의 기능으로 작품의 완성도를 높이고 나만의 개성을 더할 수 있어요."
  image={{ src: 'https://typie.net/opengraph/default.png', size: 'large' }}
  struct={{ '@context': 'https://schema.org', '@type': 'WebSite', name: '타이피', alternateName: 'typie', url: 'https://typie.co/' }}
  title="타이피 - 쓰고, 공유하고, 정리하는 글쓰기 공간"
  trailing={null}
/>

<div class={center({ flexDirection: 'column', position: 'fixed', top: '0', insetX: '0', zIndex: '40', pointerEvents: 'none' })}>
  <Toolbar {doc} {editor} sticked={true}>
    <div class={flex({ flexDirection: 'column', gap: '14px' })}>
      <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
        <Logo class={css({ height: '24px' })} />

        <Button href={env.PUBLIC_AUTH_URL} size="sm" type="link">시작하기</Button>
      </div>

      <div class={css({ marginX: '-14px', height: '1px', backgroundColor: 'gray.200' })}></div>
    </div>
  </Toolbar>
</div>

<div
  style:--grid-line-color={token('colors.gray.100')}
  style:--cross-line-color={token('colors.gray.50')}
  style:--grid-size="30px"
  style:--line-thickness="1px"
  class={flex({
    flexDirection: 'column',
    alignItems: 'center',
    width: 'screen',
    height: 'screen',
    backgroundColor: 'white',
    backgroundImage:
      '[repeating-linear-gradient(0deg, transparent, transparent calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) var(--grid-size)), repeating-linear-gradient(90deg, transparent, transparent calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) var(--grid-size)), repeating-linear-gradient(0deg, transparent, transparent calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2), transparent calc(var(--grid-size) / 2), transparent var(--grid-size)), repeating-linear-gradient(90deg, transparent, transparent calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2), transparent calc(var(--grid-size) / 2), transparent var(--grid-size))]',
    backgroundSize: 'var(--grid-size) var(--grid-size)',
    overflow: 'scroll',
  })}
>
  <div style:--prosemirror-max-width={`${maxWidth.current}px`} class={flex({ paddingTop: '240px', width: 'full', maxWidth: '1000px' })}>
    {#if browser}
      <TiptapEditor style={css.raw({ width: 'full' })} {awareness} {doc} bind:editor />
    {:else}
      <TiptapRenderer style={css.raw({ width: 'full', paddingBottom: '80px' })} content={$query.welcome.body} />
    {/if}
  </div>
</div>
