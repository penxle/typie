<script lang="ts">
  import { getAppContext } from '@typie/ui/context';
  import { page } from '$app/state';
  import { hydrateQuery } from '$lib/graphql';
  import WidgetGroup from '../@widgets/WidgetGroup.svelte';
  import { getPaneGroup } from './@pane/context.svelte';
  import Panes from './@pane/Panes.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const app = getAppContext();
  const paneGroup = getPaneGroup();

  const slug = $derived(page.params.slug);

  const root = $derived.by(() => paneGroup.state.current.root);

  // URL → PaneGroup 동기화 (single source of truth: PaneGroup)
  // hydrateQuery이므로 data는 항상 현재 slug에 대한 최신 데이터
  $effect(() => {
    if (!slug || !query.data) return;

    if (slug === 'home') {
      paneGroup.handleNavigation('home');
      return;
    }

    const siteId = query.data.entity?.site?.id;
    paneGroup.handleNavigation(slug, siteId);
  });

  // app.state 동기화 (sidebar 하이라이트 등)
  $effect(() => {
    if (slug === 'home') {
      app.state.current = undefined;
      app.state.ancestors = [];
    } else if (slug) {
      app.state.current = slug;
    }
  });

  const loaded = $derived(!!query.data && !query.loading);
</script>

{#if slug && root}
  <Panes {root} />
{/if}

{#if loaded && query.data}
  <WidgetGroup query$key={query.data} />
{/if}
