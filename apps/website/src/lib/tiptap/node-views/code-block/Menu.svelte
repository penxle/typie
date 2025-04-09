<script lang="ts">
  import { matchSorter } from 'match-sorter';
  import { bundledLanguagesInfo } from 'shiki';
  import { tick } from 'svelte';
  import IconCheck from '~icons/lucide/check';
  import IconChevronDown from '~icons/lucide/chevron-down';
  import IconChevronUp from '~icons/lucide/chevron-up';
  import IconSearch from '~icons/lucide/search';
  import { createFloatingActions } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import type { NodeViewProps } from '../../lib';

  type Props = {
    open?: boolean;
    node: NodeViewProps['node'];
    updateAttributes: NodeViewProps['updateAttributes'];
  };

  let { open = $bindable(false), node, updateAttributes }: Props = $props();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

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
    placement: 'bottom',
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
  class={center({
    gap: '4px',
    borderRadius: '4px',
    paddingX: '8px',
    paddingY: '1px',
    fontSize: '13px',
    fontWeight: 'medium',
    color: 'gray.600',
    userSelect: 'none',
    _hover: {
      backgroundColor: 'gray.200',
    },
    _expanded: {
      backgroundColor: 'gray.200',
    },
  })}
  aria-expanded={open}
  onclick={() => (open = true)}
  type="button"
  use:anchor
>
  {languages.find((language) => language.id === attrs.language)?.name ?? '?'}

  {#if open}
    <Icon style={css.raw({ color: 'gray.500', '& *': { strokeWidth: '[1.5]' } })} icon={IconChevronUp} size={14} />
  {:else}
    <Icon style={css.raw({ color: 'gray.500', '& *': { strokeWidth: '[1.5]' } })} icon={IconChevronDown} size={14} />
  {/if}
</button>

<svelte:window onkeydown={handleKeyDown} />

{#if open}
  <div
    class={flex({
      direction: 'column',
      position: 'relative',
      backgroundColor: 'white',
      borderWidth: '1px',
      borderRadius: '12px',
      maxHeight: '360px',
      overflowY: 'auto',
      scrollbar: 'hidden',
      zIndex: '50',
      boxShadow: '[0 0 0 1px rgba(0,0,0,0.05), 0 10px 20px -5px rgba(0,0,0,0.1)]',
    })}
    role="menu"
    use:floating
  >
    <div
      class={css({ padding: '8px', backgroundColor: 'white', borderTopRadius: '12px', borderBottom: '1px solid', borderColor: 'gray.100' })}
    >
      <label
        class={flex({
          align: 'center',
          gap: '8px',
          paddingY: '6px',
          paddingX: '10px',
          borderRadius: '6px',
          borderWidth: '1px',
          borderColor: 'gray.200',
          backgroundColor: 'gray.50',
          _focusWithin: {
            borderColor: 'gray.300',
            backgroundColor: 'white',
          },
        })}
      >
        <Icon style={css.raw({ color: 'gray.400' })} icon={IconSearch} size={14} />
        <input
          bind:this={inputElem}
          class={css({
            fontSize: '13px',
            width: 'full',
            backgroundColor: 'transparent',
            _placeholder: { color: 'gray.400' },
          })}
          placeholder="언어를 검색하세요"
          type="text"
          bind:value={query}
        />
      </label>
    </div>
    <ul bind:this={menuEl} class={css({ padding: '6px', flex: '1', overflowY: 'auto' })}>
      {#if filteredLanguages.length > 0}
        {#each filteredLanguages as language, index (language.id)}
          <li>
            <button
              class={flex({
                align: 'center',
                justify: 'space-between',
                gap: '4px',
                paddingX: '10px',
                paddingY: '6px',
                fontSize: '13px',
                width: 'full',
                borderRadius: '6px',
                backgroundColor: {
                  base: selectedIndex === index ? 'gray.100' : 'transparent',
                  _hover: 'gray.100',
                  _focus: 'gray.100',
                  _selected: 'gray.100',
                },
                color: attrs.language === language.id ? 'brand.500' : 'gray.700',
                fontWeight: attrs.language === language.id ? 'medium' : 'normal',
              })}
              aria-pressed={attrs.language === language.id}
              onclick={() => {
                updateAttributes({ language: language.id });
                open = false;
              }}
              onpointerover={() => (selectedIndex = index)}
              tabIndex={selectedIndex === index ? 0 : -1}
              type="button"
            >
              {language.name}

              {#if attrs.language === language.id}
                <Icon style={css.raw({ color: 'brand.500', '& *': { strokeWidth: '[2]' } })} icon={IconCheck} size={14} />
              {/if}
            </button>
          </li>
        {/each}
      {:else}
        <li>
          <div class={css({ padding: '8px', fontSize: '13px', color: 'gray.400', textAlign: 'center' })}>검색 결과가 없습니다</div>
        </li>
      {/if}
    </ul>
  </div>
{/if}
