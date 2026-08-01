<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon, Modal } from '@typie/ui/components';
  import { comma, debounce } from '@typie/ui/utils';
  import { tick } from 'svelte';
  import { match } from 'ts-pattern';
  import SearchIcon from '~icons/lucide/search';
  import { beforeNavigate, goto } from '$app/navigation';
  import { paymentInvoiceStateLabels, subscriptionStateLabels } from '$lib/admin-labels';
  import { adminSearchResultHref, parseAdminSearchQuery } from '$lib/admin-search';
  import { graphql } from '$mearie';

  type Props = {
    open: boolean;
  };

  let { open = $bindable(false) }: Props = $props();

  let listEl = $state<HTMLDivElement>();

  let query = $state('');
  let debouncedQuery = $state('');
  let selectedIndex = $state<number | null>(null);

  const normalizeSearchInput = (raw: string): string => {
    const intent = parseAdminSearchQuery(raw.toUpperCase());
    return intent?.kind === 'id' ? intent.id : raw;
  };

  const normalizedQuery = $derived(normalizeSearchInput(debouncedQuery));

  const searchQuery = createQuery(
    graphql(`
      query AdminCommandPalette_Search_Query($query: String!) {
        adminSearch(query: $query) {
          __typename

          ... on User {
            id
            name
            email
          }

          ... on Entity {
            id

            user {
              id
              name
            }

            node {
              __typename

              ... on Document {
                title
              }

              ... on Folder {
                name
              }
            }
          }

          ... on PaymentInvoice {
            id
            amount
            invoiceState: state

            user {
              id
              name
            }
          }

          ... on Subscription_ {
            id
            subscriptionState: state

            user {
              id
            }
          }

          ... on Site {
            id
            name

            user {
              id
            }
          }
        }
      }
    `),
    () => ({ query: normalizedQuery }),
    () => ({ skip: debouncedQuery.trim().length === 0 }),
  );

  const results = $derived(searchQuery.data?.adminSearch ?? []);
  type SearchResult = (typeof results)[number];

  const typeLabels: Record<SearchResult['__typename'], string> = {
    User: '유저',
    Entity: '엔티티',
    PaymentInvoice: '인보이스',
    Subscription_: '구독',
    Site: '사이트',
  };

  const labelOf = (result: SearchResult) =>
    match(result)
      .with({ __typename: 'User' }, (r) => `${r.name} · ${r.email}`)
      .with({ __typename: 'Entity' }, (r) =>
        match(r.node)
          .with({ __typename: 'Document' }, (node) => `${node.title} · ${r.user.name}`)
          .with({ __typename: 'Folder' }, (node) => `${node.name} · ${r.user.name}`)
          .otherwise(() => `(제목 없음) · ${r.user.name}`),
      )
      .with({ __typename: 'PaymentInvoice' }, (r) => `${comma(r.amount)}원 · ${r.user.name} · ${paymentInvoiceStateLabels[r.invoiceState]}`)
      .with({ __typename: 'Subscription_' }, (r) => subscriptionStateLabels[r.subscriptionState])
      .with({ __typename: 'Site' }, (r) => r.name)
      .exhaustive();

  const debouncedSetQuery = debounce((value: string) => {
    debouncedQuery = value;
    selectedIndex = null;
  }, 300);

  $effect(() => {
    if (query.trim().length === 0) {
      debouncedSetQuery.cancel();
      debouncedQuery = '';
      selectedIndex = null;
    } else {
      debouncedSetQuery.call(query);
    }
  });

  const selectResult = (result: SearchResult) => {
    goto(adminSearchResultHref(result));
    close();
  };

  const close = () => {
    open = false;

    query = '';
    debouncedQuery = '';
    selectedIndex = null;
    debouncedSetQuery.cancel();
  };

  const handleKeyDown = async (event: KeyboardEvent) => {
    const metaOrCtrlKeyOnly = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey);
    if (metaOrCtrlKeyOnly && event.key === 'k') {
      event.preventDefault();
      open = !open;
      return;
    }

    if (!open) {
      return;
    }

    if (event.key === 'Enter') {
      event.preventDefault();

      const result = results[selectedIndex ?? 0];
      if (result) {
        selectResult(result);
      }

      return;
    }

    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();

      if (results.length === 0) {
        return;
      }

      if (selectedIndex === null) {
        selectedIndex = 0;
      } else if (event.key === 'ArrowDown') {
        selectedIndex = (selectedIndex + 1) % results.length;
      } else {
        selectedIndex = (selectedIndex - 1 + results.length) % results.length;
      }

      await tick();
      const selectedElem = listEl?.querySelector<HTMLElement>(':scope > [aria-selected="true"]');

      if (
        selectedElem &&
        listEl &&
        (selectedElem.offsetTop < listEl.scrollTop ||
          selectedElem.offsetTop + selectedElem.clientHeight > listEl.scrollTop + listEl.clientHeight)
      ) {
        selectedElem.scrollIntoView({ block: 'nearest' });
      }
    }
  };

  beforeNavigate(() => {
    close();
  });
</script>

<svelte:window onkeydown={handleKeyDown} />

<Modal
  style={css.raw({ maxWidth: '600px', height: '480px', borderRadius: '14px', backgroundColor: 'admin.card.default' })}
  onclose={close}
  {open}
>
  <div class={flex({ position: 'relative', alignItems: 'center', marginX: '12px', marginY: '12px' })}>
    <input
      class={css({
        width: 'full',
        paddingLeft: '40px',
        paddingRight: '80px',
        paddingY: '6px',
        fontSize: '15px',
        fontWeight: 'medium',
        color: 'text.default',
      })}
      aria-live={query ? 'polite' : 'off'}
      onkeydown={(e) => {
        if (!((e.key === 'ArrowDown' || e.key === 'ArrowUp') && e.isComposing)) {
          return;
        }

        e.preventDefault();
        e.stopPropagation();
      }}
      placeholder="ID, 이메일, 이름, 제목, slug, permalink 무엇이든"
      tabindex="0"
      type="text"
      bind:value={query}
    />

    <div class={center({ position: 'absolute', left: '8px', top: '1/2', translate: 'auto', translateY: '-1/2', pointerEvents: 'none' })}>
      <Icon style={css.raw({ color: 'text.disabled' })} icon={SearchIcon} size={18} />
    </div>

    <div
      class={center({
        position: 'absolute',
        right: '8px',
        top: '1/2',
        translate: 'auto',
        translateY: '-1/2',
        pointerEvents: 'none',
      })}
    >
      <kbd
        class={center({
          gap: '2px',
          borderRadius: '4px',
          paddingX: '6px',
          paddingY: '2px',
          fontFamily: 'mono',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.faint',
          backgroundColor: 'surface.muted',
        })}
      >
        <span>{navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}</span>
        {#if !navigator.platform.includes('Mac')}
          <span>+</span>
        {/if}
        <span>K</span>
      </kbd>
    </div>
  </div>

  <div class={css({ height: '1px', backgroundColor: 'border.subtle' })}></div>

  <div bind:this={listEl} class={flex({ flexDirection: 'column', flexGrow: '1', paddingX: '12px', paddingY: '4px', overflowY: 'auto' })}>
    {#if debouncedQuery.trim().length === 0}
      <div class={center({ flexDirection: 'column', flexGrow: '1', width: 'full', color: 'text.disabled', fontSize: '13px' })}>
        검색어를 입력하세요
      </div>
    {:else if results.length === 0}
      <div class={center({ flexDirection: 'column', flexGrow: '1', width: 'full', color: 'text.disabled', fontSize: '13px' })}>
        결과가 없습니다
      </div>
    {:else}
      {#each results as result, idx (`${result.__typename}:${idx}`)}
        <button
          class={flex({
            alignItems: 'center',
            gap: '10px',
            borderRadius: '8px',
            paddingX: '8px',
            paddingY: '8px',
            fontSize: '13px',
            _hover: { backgroundColor: 'admin.card.hover' },
            _selected: { backgroundColor: 'admin.card.hover' },
            _focus: { backgroundColor: 'admin.card.hover' },
          })}
          aria-selected={selectedIndex === idx}
          onclick={() => selectResult(result)}
          onfocus={() => (selectedIndex = idx)}
          role="option"
          tabindex="0"
          type="button"
        >
          <span class={css({ color: 'text.disabled', fontSize: '11px', width: '64px', flexShrink: '0', textAlign: 'left' })}>
            {typeLabels[result.__typename]}
          </span>
          <span class={css({ color: 'text.default', truncate: true })}>{labelOf(result)}</span>
        </button>
      {/each}
    {/if}
  </div>

  <div class={css({ height: '1px', backgroundColor: 'border.subtle' })}></div>

  <div
    class={flex({
      alignItems: 'center',
      gap: '16px',
      paddingX: '12px',
      paddingY: '10px',
      fontSize: '12px',
      color: 'text.faint',
      backgroundColor: 'surface.muted',
    })}
  >
    <span>↑↓ 이동</span>
    <span>Enter 선택</span>
    <span>Esc 닫기</span>
  </div>
</Modal>
