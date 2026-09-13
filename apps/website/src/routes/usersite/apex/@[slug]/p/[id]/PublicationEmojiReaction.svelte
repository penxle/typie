<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex, grid } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import mixpanel from 'mixpanel-browser';
  import { fade } from 'svelte/transition';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import SmilePlusIcon from '~icons/lucide/smile-plus';
  import { emojis } from '$lib/usersite/emoji';
  import Emoji from '$lib/usersite/Emoji.svelte';
  import { graphql } from '$mearie';
  import type { UsersiteSpacePublicationPage_PublicationEmojiReaction_publicationView$key } from '$mearie';

  type Props = {
    publicationView$key: UsersiteSpacePublicationPage_PublicationEmojiReaction_publicationView$key;
  };

  let { publicationView$key }: Props = $props();

  const publication = createFragment(
    graphql(`
      fragment UsersiteSpacePublicationPage_PublicationEmojiReaction_publicationView on PublicationView {
        id
        documentId
        allowReaction

        reactions {
          id
          emoji
        }
      }
    `),
    () => publicationView$key,
  );

  const [createDocumentReaction] = createMutation(
    graphql(`
      mutation UsersiteSpacePublicationPage_PublicationEmojiReaction_CreateDocumentReaction_Mutation($input: CreateDocumentReactionInput!) {
        createDocumentReaction(input: $input) {
          id

          publication {
            id

            reactions {
              id
              emoji
            }
          }
        }
      }
    `),
  );

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

{#if publication.data.allowReaction}
  <button
    class={css({
      marginTop: '2px',
      borderRadius: '4px',
      padding: '3px',
      color: 'text.muted',
      _hover: { backgroundColor: 'surface.hover' },
      _expanded: { color: 'text.default', backgroundColor: 'surface.active' },
    })}
    aria-expanded={open}
    aria-haspopup="dialog"
    onclick={() => {
      open = true;
      mixpanel.track('open_document_reaction_popover');
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
        borderColor: 'border.hairline',
        borderRadius: '6px',
        padding: '4px',
        backgroundColor: 'surface.default',
        boxShadow: 'sm',
      })}
      use:floating
      transition:fade={{ duration: 100 }}
    >
      {#each Object.keys(emojis) as emoji (emoji)}
        <li>
          <button
            class={center({ borderRadius: '4px', padding: '5px', size: 'full', _supportHover: { backgroundColor: 'surface.hover' } })}
            onclick={async () => {
              await createDocumentReaction({ input: { documentId: publication.data.documentId, emoji } });
              mixpanel.track('create_document_reaction', { emoji });
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
    {#each showAll ? publication.data.reactions : publication.data.reactions.slice(0, MAX_REACTIONS) as reaction (reaction.id)}
      <Emoji emoji={reaction.emoji} />
    {/each}

    {#if publication.data.reactions.length > MAX_REACTIONS}
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
            {publication.data.reactions.length - MAX_REACTIONS}
          {/if}
        </button>
      </li>
    {/if}
  </ul>
{/if}
