<script lang="ts">
  import HeartPlusIcon from '~icons/lucide/heart';
  import { fragment, graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Icon } from '$lib/components';
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
  <button onclick={() => (open = true)} type="button" use:anchor>
    <Icon icon={HeartPlusIcon} />
  </button>

  {#if open}
    <ul
      class={grid({
        columns: 5,
        borderWidth: '1px',
        borderColor: 'gray.200',
        borderRadius: '12px',
        padding: '8px',
        backgroundColor: 'white',
        boxShadow: 'small',
      })}
      use:floating
    >
      {#each Object.keys(emojis) as emoji (emoji)}
        <li>
          <button
            class={center({ borderRadius: '4px', padding: '2px', size: 'full', _supportHover: { backgroundColor: 'gray.200' } })}
            onclick={async () => {
              await createPostReaction({ postId: $postView.id, emoji });
            }}
            type="button"
          >
            <Emoji {emoji} />
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <ul class={flex({ align: 'center', gap: '4px', wrap: 'wrap' })}>
    {#each $postView.reactions as reaction (reaction.id)}
      <Emoji emoji={reaction.emoji} />
    {/each}
  </ul>
{/if}
