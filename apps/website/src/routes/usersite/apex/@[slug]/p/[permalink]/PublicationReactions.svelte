<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import mixpanel from 'mixpanel-browser';
  import { cubicOut } from 'svelte/easing';
  import { scale } from 'svelte/transition';
  import { emojis } from '$lib/usersite/emoji';
  import Emoji from '$lib/usersite/Emoji.svelte';
  import { graphql } from '$mearie';
  import type { UsersiteSpacePublicationPage_PublicationReactions_publicationView$key } from '$mearie';

  type Props = {
    publicationView$key: UsersiteSpacePublicationPage_PublicationReactions_publicationView$key;
  };

  let { publicationView$key }: Props = $props();

  const publication = createFragment(
    graphql(`
      fragment UsersiteSpacePublicationPage_PublicationReactions_publicationView on PublicationView {
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
      mutation UsersiteSpacePublicationPage_PublicationReactions_CreateDocumentReaction_Mutation($input: CreateDocumentReactionInput!) {
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

  const ROTATIONS = [-7, 4, -2, 8, -5, 2, 6, -8, 3, -3, 7, -6, 1, 5, -4];
  const CELL = 30;
  const ROWS = 2;
  const MORE_CELLS = 2;

  let listWidth = $state(0);
  let showAll = $state(false);

  const reactions = $derived(publication.data.reactions);
  const perRow = $derived(Math.max(1, Math.floor((listWidth + 2) / CELL)));
  const cap = $derived(perRow * ROWS);
  const truncated = $derived(!showAll && reactions.length > cap);
  const shown = $derived(truncated ? reactions.slice(0, Math.max(1, cap - MORE_CELLS)) : reactions);
  const hiddenCount = $derived(reactions.length - shown.length);

  const react = async (emoji: string) => {
    await createDocumentReaction({ input: { documentId: publication.data.documentId, emoji } });
    mixpanel.track('create_document_reaction', { emoji });
  };
</script>

{#if publication.data.allowReaction}
  <div
    class={css({
      position: 'relative',
      minHeight: '60px',
      paddingX: '16px',
      paddingY: '20px',
      borderWidth: '1px',
      borderStyle: 'dashed',
      borderColor: 'border.default',
      borderRadius: '12px',
    })}
  >
    <div
      class={flex({
        position: 'absolute',
        top: '-20px',
        right: '16px',
        alignItems: 'center',
        gap: '2px',
        padding: '3px',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '12px',
        backgroundColor: 'surface.default',
      })}
      aria-label="반응 남기기"
      role="group"
    >
      {#each Object.keys(emojis) as emoji (emoji)}
        <button
          class={css({
            position: 'relative',
            display: 'grid',
            placeItems: 'center',
            size: '30px',
            borderRadius: '8px',
            transition: 'common',
            _hover: { backgroundColor: 'surface.hover', '& [data-plus]': { opacity: '100', transform: 'scale(1)' } },
            _active: { transform: 'scale(0.92)' },
          })}
          aria-label={emoji}
          onclick={() => react(emoji)}
          type="button"
        >
          <Emoji {emoji} size={18} />
          <span
            class={css({
              position: 'absolute',
              top: '-1px',
              right: '-1px',
              display: 'grid',
              placeItems: 'center',
              size: '13px',
              borderRadius: 'full',
              backgroundColor: 'text.default',
              color: 'surface.default',
              fontSize: '10px',
              fontWeight: 'bold',
              lineHeight: '[1]',
              boxShadow: '[0 0 0 2px token(colors.surface.hover)]',
              opacity: '0',
              transform: 'scale(0.6)',
              transition: '[opacity 120ms ease-out, transform 160ms cubic-bezier(0.23, 1, 0.32, 1)]',
            })}
            aria-hidden="true"
            data-plus
          >
            +
          </span>
        </button>
      {/each}
    </div>

    {#if reactions.length > 0}
      <ul
        class={flex({ flexWrap: 'wrap', alignItems: 'center', columnGap: '2px', rowGap: '4px', marginLeft: '-3px' })}
        aria-label={`반응 ${reactions.length}개`}
        bind:clientWidth={listWidth}
      >
        {#each shown as reaction, index (reaction.id)}
          <li
            style:--rotate={`${ROTATIONS[index % ROTATIONS.length]}deg`}
            class={css({ display: 'grid', placeItems: 'center', size: '28px', transform: '[rotate(var(--rotate))]' })}
            in:scale={{ start: 0.6, duration: 280, easing: cubicOut }}
          >
            <Emoji emoji={reaction.emoji} size={20} />
          </li>
        {/each}

        {#if truncated || showAll}
          <li class={css({ display: 'grid', placeItems: 'center', marginLeft: '4px' })}>
            <button
              class={css({
                height: '28px',
                paddingX: '9px',
                borderWidth: '1px',
                borderColor: 'border.hairline',
                borderRadius: 'full',
                backgroundColor: 'surface.default',
                fontSize: '12px',
                color: 'text.muted',
                fontVariantNumeric: 'tabular-nums',
                transition: 'common',
                _hover: { color: 'text.default', borderColor: 'border.default' },
              })}
              onclick={() => (showAll = !showAll)}
              type="button"
            >
              {showAll ? '접기' : `+${hiddenCount}`}
            </button>
          </li>
        {/if}
      </ul>
    {:else}
      <p class={css({ fontSize: '13px', lineHeight: '[28px]', color: 'text.muted' })}>반응을 달아 작가에게 응원을 남겨보세요</p>
    {/if}
  </div>
{/if}
