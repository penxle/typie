<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Icon, Marquee } from '@typie/ui/components';
  import { z } from 'zod';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import { goto } from '$app/navigation';
  import { graphql } from '$mearie';
  import EntityIcon from '../../@context-menu/EntityIcon.svelte';
  import PrismBrokenCard from './PrismBrokenCard.svelte';
  import type { MarkdownCardProps } from './index.ts';

  let { text, pending, settled }: MarkdownCardProps = $props();

  const schema = z.object({ id: z.string() });

  const parse = (raw: string): unknown => {
    try {
      return JSON.parse(raw);
    } catch {
      return null;
    }
  };

  const payload = $derived(schema.safeParse(parse(text)));

  const query = createQuery(
    graphql(`
      query DashboardLayout_PrismDocumentCard_Query($documentId: ID!) {
        documentById(documentId: $documentId) {
          id
          nullableTitle
          subtitle
          characterCount

          entity {
            id
            slug
            icon
            iconColor
          }
        }
      }
    `),
    () => ({ documentId: payload.success ? payload.data.id : '' }),
    () => ({ skip: pending || !payload.success }),
  );

  const doc = $derived(query.data?.documentById ?? null);
  const title = $derived(doc?.nullableTitle ?? '제목 없음');
  const getCard = (element: HTMLElement) => element.closest<HTMLElement>('button');

  const frameStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    height: '40px',
    paddingX: '12px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '10px',
    fontSize: '13px',
    backgroundColor: 'surface.default',
    _dark: { backgroundColor: 'surface.subtle' },
    boxShadow: 'small',
  });
  const titleClass = css({ display: 'flex', alignItems: 'center', gap: '6px', minWidth: '0' });
</script>

{#snippet skeleton()}
  <div class={css(frameStyle)}>
    <div
      class={css({
        height: '14px',
        width: '[60%]',
        borderRadius: '4px',
        backgroundColor: 'surface.muted',
        animation: 'pulse 1.6s ease-in-out infinite',
      })}
    ></div>
  </div>
{/snippet}

{#if pending && settled}
  <PrismBrokenCard />
{:else if pending}
  {@render skeleton()}
{:else if !payload.success}
  <PrismBrokenCard />
{:else if query.error}
  <PrismBrokenCard message="문서를 찾을 수 없어요" retry={() => query.refetch()} />
{:else if doc === null}
  {@render skeleton()}
{:else}
  <button
    class={css(frameStyle, {
      width: 'full',
      textAlign: 'left',
      cursor: 'pointer',
      transition: '[background-color 150ms ease]',
      _hover: { backgroundColor: 'surface.muted' },
    })}
    onclick={() => void goto(`/${doc.entity.slug}`)}
    type="button"
  >
    <span class={titleClass}>
      <EntityIcon style={css.raw({ flexShrink: '0' })} icon={doc.entity.icon} iconColor={doc.entity.iconColor} size={14} />
      <Marquee class={css({ minWidth: '0', fontWeight: 'semibold' })} bleed={8} fogSize={16} getTrigger={getCard} text={title} />
    </span>
    <span
      class={css({
        flexShrink: '0',
        maxWidth: '[160px]',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap',
        fontSize: '11px',
        color: 'text.faint',
      })}
    >
      {doc.subtitle ?? `${doc.characterCount.toLocaleString()}자`}
    </span>
    <Icon style={css.raw({ flexShrink: '0', marginLeft: 'auto', color: 'text.faint' })} icon={ChevronRightIcon} size={14} />
  </button>
{/if}
