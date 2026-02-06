<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex, grid } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import mixpanel from 'mixpanel-browser';
  import { fade } from 'svelte/transition';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import SmilePlusIcon from '~icons/lucide/smile-plus';
  import { fragment, graphql } from '$graphql';
  import { emojis } from './emoji';
  import Emoji from './Emoji.svelte';
  import type { UsersiteWildcardSlugPage_DocumentEmojiReaction_documentView } from '$graphql';

  type Props = {
    $documentView: UsersiteWildcardSlugPage_DocumentEmojiReaction_documentView;
  };

  let { $documentView: _documentView }: Props = $props();

  const documentView = fragment(
    _documentView,
    graphql(`
      fragment UsersiteWildcardSlugPage_DocumentEmojiReaction_documentView on DocumentView {
        id
        allowReaction

        reactions {
          id
          emoji
        }
      }
    `),
  );

  const createDocumentReaction = graphql(`
    mutation UsersiteWildcardSlugPage_DocumentEmojiReaction_CreateDocumentReaction_Mutation($input: CreateDocumentReactionInput!) {
      createDocumentReaction(input: $input) {
        id

        document {
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

{#if $documentView.allowReaction}
  <button
    class={css({ marginTop: '2px', borderRadius: '4px', padding: '3px', _hover: { backgroundColor: 'surface.muted' } })}
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
              await createDocumentReaction({ documentId: $documentView.id, emoji });
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
    {#each showAll ? $documentView.reactions : $documentView.reactions.slice(0, MAX_REACTIONS) as reaction (reaction.id)}
      <Emoji emoji={reaction.emoji} />
    {/each}

    {#if $documentView.reactions.length > MAX_REACTIONS}
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
            {$documentView.reactions.length - MAX_REACTIONS}
          {/if}
        </button>
      </li>
    {/if}
  </ul>
{/if}
