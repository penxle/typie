<script lang="ts">
  import { matchSorter } from 'match-sorter';
  import { bundledLanguagesInfo } from 'shiki';
  import { tick } from 'svelte';
  import IconCheck from '~icons/lucide/check';
  import IconChevronDown from '~icons/lucide/chevron-down';
  import IconChevronUp from '~icons/lucide/chevron-up';
  import IconSearch from '~icons/lucide/search';
  import { createFloatingActions } from '$lib/actions';
  import { HorizontalDivider, Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { NodeViewProps } from '../../lib';

  type Props = {
    open?: boolean;
    node: NodeViewProps['node'];
    updateAttributes: NodeViewProps['updateAttributes'];
  };

  let { open = $bindable(false), node, updateAttributes }: Props = $props();

  let query = $state('');
  let inputElem = $state<HTMLInputElement>();
  let buttonEl = $state<HTMLButtonElement>();
  let menuEl = $state<HTMLUListElement>();
  let selectedIndex = $state<number | null>(null);

  $effect(() => {
    if (open) {
      tick().then(() => {
        inputElem?.focus();
      });
    }
  });

  const close = () => {
    open = false;
    query = '';
    buttonEl?.focus();
  };

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom-end',
    offset: 4,
    onClickOutside: close,
  });

  const languages = [
    ...bundledLanguagesInfo.map((language) => ({ id: language.id, name: language.name, aliases: language.aliases })),
    { id: 'text', name: 'Plain Text', aliases: [] },
  ].toSorted((a, b) => a.name.localeCompare(b.name));

  const filteredLanguages = $derived(
    matchSorter(languages, query, {
      keys: ['name', 'aliases'],
      sorter: (items) => items,
    }),
  );

  const handleKeyDown = (e: KeyboardEvent) => {
    const target = e.target as HTMLElement;
    if (open) {
      if (e.key === 'Escape') {
        e.preventDefault();
        close();
        return;
      }

      if (e.key === 'Tab') {
        close();
        return;
      }

      if (e.key === 'Enter') {
        e.preventDefault();
        if (selectedIndex !== null) {
          const language = filteredLanguages[selectedIndex];
          if (language) {
            updateAttributes({ language: language.id });
            close();
          }
        }
      }

      if (e.key === 'ArrowDown') {
        selectedIndex = Math.min(selectedIndex ?? -1, filteredLanguages.length - 2) + 1;
      } else if (e.key === 'ArrowUp') {
        selectedIndex = Math.max((selectedIndex ?? filteredLanguages.length) - 1, 0);
      }

      if (selectedIndex !== null) {
        const menuItems = menuEl?.querySelectorAll('button');
        if (!menuItems || menuItems.length === 0) {
          return;
        }
        menuItems[selectedIndex]?.scrollIntoView({ behavior: 'auto', block: 'nearest' });
      }
    } else {
      const focusInButton = buttonEl?.contains(target);
      if (focusInButton && e.key === 'ArrowDown') {
        e.preventDefault();
        open = true;
      }
    }
  };
</script>

<button
  bind:this={buttonEl}
  class={css({
    display: 'flex',
    alignItems: 'center',
    gap: '4px',
    fontSize: '14px',
    fontWeight: 'semibold',
    paddingLeft: '10px',
    paddingRight: '6px',
    paddingY: '2px',
    borderRadius: '4px',
    color: 'gray.700',
    _hover: {
      backgroundColor: 'gray.200',
    },
  })}
  onclick={() => (open = true)}
  type="button"
  use:anchor
>
  {languages.find((language) => language.id === node.attrs.language)?.name ?? '?'}

  {#if open}
    <Icon icon={IconChevronUp} />
  {:else}
    <Icon icon={IconChevronDown} />
  {/if}
</button>

<svelte:window onkeydown={handleKeyDown} />

{#if open}
  <div
    class={flex({
      direction: 'column',
      position: 'relative',
      backgroundColor: 'white',
      borderRadius: '8px',
      maxHeight: '360px',
      overflowY: 'auto',
      scrollbar: 'hidden',
      zIndex: '50',
      boxShadow: 'xlarge',
    })}
    role="menu"
    use:floating
  >
    <div class={css({ padding: '8px', backgroundColor: 'white' })}>
      <label
        class={flex({
          align: 'center',
          gap: '8px',
          paddingY: '7px',
          paddingX: '10px',
          borderRadius: '4px',
          borderWidth: '1px',
        })}
      >
        <Icon style={css.raw({})} icon={IconSearch} size={14} />
        <input bind:this={inputElem} class={css({ fontSize: '14px' })} placeholder="언어를 검색하세요" type="text" bind:value={query} />
      </label>
    </div>
    <HorizontalDivider />
    <ul bind:this={menuEl} class={css({ padding: '8px', flex: '1', overflowY: 'auto' })}>
      {#if filteredLanguages.length > 0}
        {#each filteredLanguages as language, index (language.id)}
          <li>
            <button
              class={flex({
                align: 'center',
                justify: 'space-between',
                gap: '4px',
                paddingX: '14px',
                paddingY: '6px',
                fontSize: '14px',
                width: 'full',
                borderRadius: '4px',
                backgroundColor: {
                  base: selectedIndex === index ? 'gray.100' : 'transparent',
                  _hover: 'gray.100',
                  _focus: 'gray.100',
                  _selected: 'gray.100',
                },
              })}
              aria-pressed={node.attrs.language === language.id}
              onclick={() => {
                updateAttributes({ language: language.id });
                open = false;
              }}
              onpointerover={() => (selectedIndex = index)}
              tabIndex={selectedIndex === index ? 0 : -1}
              type="button"
            >
              {language.name}

              {#if node.attrs.language === language.id}
                <Icon style={css.raw({ color: 'brand.400', '& *': { strokeWidth: '[2]' } })} icon={IconCheck} />
              {/if}
            </button>
          </li>
        {/each}
      {:else}
        <li>
          <div class={css({ padding: '8px', fontSize: '14px', color: 'gray.500' })}>검색 결과가 없습니다</div>
        </li>
      {/if}
    </ul>
  </div>
{/if}
