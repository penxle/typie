<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { getAppContext } from '$lib/context/app.svelte';
  import type { DashboardLayout_Shortcuts_query } from '$graphql';

  type Props = {
    $query: DashboardLayout_Shortcuts_query;
  };

  let { $query: _query }: Props = $props();

  const app = getAppContext();

  const query = fragment(
    _query,
    graphql(`
      fragment DashboardLayout_Shortcuts_query on Query {
        me @required {
          id
          sites {
            id
          }
        }
      }
    `),
  );

  const createPost = graphql(`
    mutation Shortcuts_CreatePost_Mutation($input: CreatePostInput!) {
      createPost(input: $input) {
        id
        entity {
          id
          slug
        }
      }
    }
  `);

  const handleKeydown = async (event: KeyboardEvent) => {
    if (event.altKey && event.code === 'KeyN') {
      event.preventDefault();
      if (event.key === 'Dead') {
        (event.target as HTMLElement)?.blur();
      }

      const siteId = $query.me.sites[0].id;
      const resp = await createPost({ siteId, parentEntityId: app.state.ancestors.at(-1) });

      mixpanel.track('create_post', { via: 'shortcut' });
      await goto(`/${resp.entity.slug}`);

      return;
    }

    if (event.altKey && event.code === 'KeyT') {
      event.preventDefault();

      if (app.preference.current.postsExpanded === false) {
        app.state.postsOpen = !app.state.postsOpen;
      } else {
        app.preference.current.postsExpanded = app.preference.current.postsExpanded === 'open' ? 'closed' : 'open';
      }

      return;
    }

    if (event.altKey && event.code === 'KeyP') {
      event.preventDefault();

      app.preference.current.panelExpanded = !app.preference.current.panelExpanded;

      return;
    }

    if (event.shiftKey && event.altKey && event.code === 'KeyZ') {
      event.preventDefault();

      app.preference.current.zenModeEnabled = !app.preference.current.zenModeEnabled;

      return;
    }

    if (event.code === 'Escape' && app.preference.current.zenModeEnabled) {
      event.preventDefault();

      app.preference.current.zenModeEnabled = false;

      return;
    }
  };
</script>

<svelte:window onkeydown={handleKeydown} />
