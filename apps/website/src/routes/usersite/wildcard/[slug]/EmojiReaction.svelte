<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex, grid } from '@typie/styled-system/patterns';
  import mixpanel from 'mixpanel-browser';
  import { fade } from 'svelte/transition';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import SmilePlusIcon from '~icons/lucide/smile-plus';
  import { fragment, graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { emojis } from './emoji';
  import Emoji from './Emoji.svelte';
  import type { UsersiteWildcardSlugPage_EmojiReaction_postView } from '$graphql';

  type Props = {
    $postView: UsersiteWildcardSlugPage_EmojiReaction_postView;
  };

  let { $postView: _postView }: Props = $props();

  const postView = fragment(
    _postView,
    graphql(`
      fragment UsersiteWildcardSlugPage_EmojiReaction_postView on PostView {
        id
        allowReaction

        reactions {
          id
          emoji
        }
      }
    `),
  );

  const createPostReaction = graphql(`
    mutation UsersiteWildcardSlugPage_EmojiReaction_CreatePostReaction_Mutation($input: CreatePostReactionInput!) {
      createPostReaction(input: $input) {
        id

        post {
          id

          reactions {
            id
            emoji
          }
        }
      }
    }
  `);

  let open = $state(false);
  let showAll = $state(false);

  const { anchor, floating } = createFloatingActions({
    placement: 'top',
    offset: 6,
    onClickOutside: () => {
      open = false;
    },
  });
  const MAX_REACTIONS = 100;
</script>

{#if $postView.allowReaction}
  <button
    class={css({ marginTop: '2px', borderRadius: '4px', padding: '3px', _hover: { backgroundColor: 'surface.muted' } })}
    onclick={() => {
      open = true;
      mixpanel.track('open_post_reaction_popover');
    }}
    type="button"
    use:anchor
  >
    <Icon icon={SmilePlusIcon} />
  </button>

  {#if open}
    <ul
      class={grid({
        columns: 5,
        gap: '6px',
        borderWidth: '1px',
        borderColor: 'border.subtle',
        borderRadius: '6px',
        padding: '4px',
        backgroundColor: 'surface.default',
        boxShadow: 'small',
      })}
      use:floating
      transition:fade={{ duration: 100 }}
    >
      {#each Object.keys(emojis) as emoji (emoji)}
        <li>
          <button
            class={center({ borderRadius: '4px', padding: '5px', size: 'full', _supportHover: { backgroundColor: 'surface.muted' } })}
            onclick={async () => {
              await createPostReaction({ postId: $postView.id, emoji });
              mixpanel.track('create_post_reaction', { emoji });
            }}
            type="button"
          >
            <Emoji {emoji} />
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <ul class={flex({ align: 'center', gap: '4px', wrap: 'wrap', marginTop: '4px' })}>
    {#each showAll ? $postView.reactions : $postView.reactions.slice(0, MAX_REACTIONS) as reaction (reaction.id)}
      <Emoji emoji={reaction.emoji} />
    {/each}

    {#if $postView.reactions.length > MAX_REACTIONS}
      <li>
        <button
          class={flex({ align: 'center', gap: '2px', fontSize: '13px', color: 'text.muted' })}
          onclick={() => (showAll = !showAll)}
          type="button"
        >
          {#if showAll}
            <Icon icon={ChevronUpIcon} size={12} />
            접기
          {:else}
            ...
            <Icon icon={ChevronDownIcon} size={12} />
            {$postView.reactions.length - MAX_REACTIONS}
          {/if}
        </button>
      </li>
    {/if}
  </ul>
{/if}
