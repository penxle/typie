<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { fade } from 'svelte/transition';
  import SmilePlusIcon from '~icons/lucide/smile-plus';
  import { fragment, graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex, grid } from '$styled-system/patterns';
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

  const { anchor, floating } = createFloatingActions({
    placement: 'top',
    offset: 6,
    onClickOutside: () => {
      open = false;
    },
  });
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
        gap: '8px',
        borderWidth: '1px',
        borderColor: 'border.subtle',
        borderRadius: '8px',
        paddingX: '8px',
        paddingY: '6px',
        backgroundColor: 'surface.default',
        boxShadow: 'small',
      })}
      use:floating
      transition:fade={{ duration: 100 }}
    >
      {#each Object.keys(emojis) as emoji (emoji)}
        <li>
          <button
            class={center({ borderRadius: '4px', padding: '3px', size: 'full', _supportHover: { backgroundColor: 'surface.muted' } })}
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
    {#each $postView.reactions as reaction (reaction.id)}
      <Emoji emoji={reaction.emoji} />
    {/each}
  </ul>
{/if}
