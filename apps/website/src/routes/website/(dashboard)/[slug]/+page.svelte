<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { getAppContext } from '@typie/ui/context';
  import { nanoid } from 'nanoid';
  import { untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { graphql } from '$mearie';
  import WidgetGroup from '../@widgets/WidgetGroup.svelte';
  import { getPaneGroup } from './@pane/context.svelte';
  import Panes from './@pane/Panes.svelte';

  const query = createQuery(
    graphql(`
      query DashboardSlugPage_Query {
        me @required {
          id
        }

        ...WidgetGroup_query
      }
    `),
  );

  const app = getAppContext();
  const paneGroup = getPaneGroup();

  const slug = $derived(page.params.slug);

  const root = $derived.by(() => paneGroup.state.current.root);
  const panes = $derived(paneGroup.panes);

  // Case 1: 외부 네비게이션 (URL → pane tree)
  $effect(() => {
    if (!slug) return;

    untrack(() => {
      const isHome = slug === 'home';

      if (isHome) {
        app.state.current = undefined;
        app.state.ancestors = [];
      } else {
        app.state.current = slug;
      }

      if (paneGroup.state.current.root) {
        if (isHome) {
          const existing = panes.find((p) => p.kind === 'home');
          if (existing) {
            paneGroup.state.current.focusedPaneId = existing.id;
          } else {
            const focusedPaneId = paneGroup.state.current.focusedPaneId;
            if (focusedPaneId) {
              paneGroup.replacePane(focusedPaneId, { kind: 'home' });
            }
          }
        } else {
          const existing = panes.find((p) => p.kind === 'entity' && p.slug === slug);
          if (existing) {
            paneGroup.state.current.focusedPaneId = existing.id;
          } else {
            const focusedPaneId = paneGroup.state.current.focusedPaneId;
            if (focusedPaneId) {
              paneGroup.replacePane(focusedPaneId, { kind: 'entity', slug });
            } else {
              const paneId = nanoid();
              paneGroup.state.current.root = {
                id: nanoid(),
                type: 'axis',
                direction: 'horizontal',
                children: [{ id: paneId, type: 'pane', kind: 'entity', slug }],
                flexes: [1],
              };
              paneGroup.state.current.focusedPaneId = paneId;
            }
          }
        }
      } else {
        const paneId = nanoid();
        paneGroup.state.current.root = {
          id: nanoid(),
          type: 'axis',
          direction: 'horizontal',
          children: [
            isHome ? { id: paneId, type: 'pane', kind: 'home' as const } : { id: paneId, type: 'pane', kind: 'entity' as const, slug },
          ],
          flexes: [1],
        };
        paneGroup.state.current.focusedPaneId = paneId;
      }
    });
  });

  // Case 2: URL 파생 동기화 (pane tree → URL)
  $effect(() => {
    const focusedPaneId = paneGroup.state.current.focusedPaneId;
    if (!focusedPaneId) return;

    const focusedPane = panes.find((p) => p.id === focusedPaneId);
    if (focusedPane?.kind === 'entity' && focusedPane.slug !== page.params.slug) {
      goto(`/${focusedPane.slug}`, { replaceState: true, keepFocus: true });
    } else if (focusedPane?.kind === 'home' && page.params.slug !== 'home') {
      goto('/home', { replaceState: true, keepFocus: true });
    }
  });

  const loaded = $derived(!!query.data && !query.loading);
</script>

{#if loaded && query.data && slug && root}
  <Panes {root} />
{:else}
  <div
    class={center({
      size: 'full',
      backgroundColor: 'surface.default',
    })}
  >
    <Logo
      class={css({
        size: '32px',
        filter: '[grayscale(100%)]',
        animation: 'pulse 2s ease-in-out infinite',
      })}
    />
  </div>
{/if}

{#if loaded && query.data}
  <WidgetGroup query$key={query.data} />
{/if}
