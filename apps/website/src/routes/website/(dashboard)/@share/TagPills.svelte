<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { createFloatingActions, registerFocusTrapContainer, tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { untrack } from 'svelte';
  import PlusIcon from '~icons/lucide/plus';
  import XIcon from '~icons/lucide/x';
  import { duplicatedTagInput, parseTagInput } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { tagChipStyle } from './publish-styles';
  import TagSuggestions from './TagSuggestions.svelte';

  type Props = {
    siteId: string;
    tags: string[];
    partial?: { tag: string; count: number }[];
    onremovepartial?: (tag: string) => void;
    disabled?: boolean;
  };

  let { siteId, tags = $bindable(), partial = [], onremovepartial, disabled = false }: Props = $props();

  type Mode = { kind: 'idle' } | { kind: 'add' } | { kind: 'edit'; tag: string };

  let mode = $state<Mode>({ kind: 'idle' });
  let draft = $state('');
  let flashed = $state<string | null>(null);
  let announcement = $state('');
  let focusedIndex = $state<number | null>(null);
  let addButtonEl = $state<HTMLButtonElement>();
  let inputEl = $state<HTMLInputElement>();
  let chipEls = $state<(HTMLButtonElement | null)[]>([]);

  const SUGGEST_DEBOUNCE_MS = 150;
  const panelId = $props.id();

  let debouncedDraft = $state('');
  let highlighted = $state<number | null>(null);
  type Suggestion = { name: string; count: number };

  let suggestions = $state<{ mine: readonly Suggestion[]; popular: readonly Suggestion[] }>({ mine: [], popular: [] });
  let panelEl = $state<HTMLDivElement>();
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  const suggestionsQuery = createQuery(
    graphql(`
      query DashboardLayout_Share_TagPills_Query($siteId: ID!, $query: String!, $exclude: [String!]) {
        site(siteId: $siteId) {
          id

          tagSuggestions(query: $query, exclude: $exclude) {
            mine {
              name
              count
            }

            popular {
              name
              count
            }
          }
        }
      }
    `),
    () => ({ siteId, query: debouncedDraft, exclude: tags }),
    () => ({ skip: mode.kind !== 'add' }),
  );

  $effect(() => {
    const data = suggestionsQuery.data?.site.tagSuggestions;
    if (data) suggestions = { mine: data.mine, popular: data.popular };
  });

  const visibleSuggestions = $derived({
    mine: suggestions.mine.filter((suggestion) => !tags.includes(suggestion.name)),
    popular: suggestions.popular.filter((suggestion) => !tags.includes(suggestion.name)),
  });
  const flatSuggestions = $derived([...visibleSuggestions.mine, ...visibleSuggestions.popular]);
  const panelOpen = $derived(mode.kind === 'add' && flatSuggestions.length > 0);

  const { anchor: suggestAnchor, floating: suggestFloating } = createFloatingActions({ placement: 'bottom-start', offset: 4 });

  $effect(() => {
    const input = inputEl;
    const panel = panelEl;
    if (!panelOpen || !input || !panel) return;
    const host = input.closest<HTMLElement>('[data-focus-trap]');
    if (!host) return;
    return untrack(() => registerFocusTrapContainer(host, panel));
  });

  $effect(() => {
    return () => {
      if (debounceTimer) clearTimeout(debounceTimer);
    };
  });

  const scheduleSuggest = (value: string) => {
    if (debounceTimer) clearTimeout(debounceTimer);
    highlighted = null;
    debounceTimer = setTimeout(() => {
      debouncedDraft = value;
    }, SUGGEST_DEBOUNCE_MS);
  };

  const resetSuggest = () => {
    if (debounceTimer) clearTimeout(debounceTimer);
    debouncedDraft = '';
    highlighted = null;
  };

  const pickSuggestion = (name: string) => {
    commit(name);
    draft = '';
    resetSuggest();
    inputEl?.focus();
  };

  const moveHighlight = (delta: 1 | -1) => {
    const total = flatSuggestions.length;
    if (total === 0) return;
    if (highlighted === null) {
      highlighted = delta === 1 ? 0 : total - 1;
      return;
    }
    highlighted = (highlighted + delta + total) % total;
  };

  let flashTimer: ReturnType<typeof setTimeout> | undefined;
  let skipBlur = false;
  let switching = false;

  $effect(() => {
    return () => {
      if (flashTimer) clearTimeout(flashTimer);
    };
  });

  const flash = (tag: string) => {
    if (flashTimer) clearTimeout(flashTimer);
    flashed = tag;
    announcement = `${tag}: 이미 추가된 태그예요`;
    flashTimer = setTimeout(() => (flashed = null), 900);
  };

  const commit = (raw: string) => {
    const duplicated = duplicatedTagInput(raw, tags);
    const next = parseTagInput(raw, tags);
    if (next.length !== tags.length) tags = next;
    if (duplicated) flash(duplicated);
  };

  const rename = (from: string, raw: string): boolean => {
    const index = tags.indexOf(from);
    if (index === -1) return false;

    const rest = tags.filter((tag) => tag !== from);
    const pieces = parseTagInput(raw, []);
    const duplicated = pieces.find((tag) => rest.includes(tag)) ?? null;
    const added = pieces.filter((tag) => !rest.includes(tag));

    tags = [...rest.slice(0, index), ...added, ...rest.slice(index)];
    if (duplicated) flash(duplicated);

    return added.length > 0;
  };

  const remove = (tag: string) => {
    tags = tags.filter((t) => t !== tag);
  };

  const focusChip = (index: number) => {
    chipEls[index]?.focus();
  };

  const focusChipLater = (index: number) => {
    requestAnimationFrame(() => focusChip(Math.max(0, Math.min(index, tags.length - 1))));
  };

  const close = () => {
    mode = { kind: 'idle' };
    draft = '';
    suggestions = { mine: [], popular: [] };
    resetSuggest();
  };

  const startAdding = () => {
    skipBlur = false;
    switching = mode.kind !== 'idle';
    mode = { kind: 'add' };
    draft = '';
    resetSuggest();

    requestAnimationFrame(() => {
      switching = false;
      inputEl?.focus();
    });
  };

  const startEditing = (tag: string) => {
    if (mode.kind === 'edit') {
      if (mode.tag === tag) return;
      rename(mode.tag, draft);
    } else if (mode.kind === 'add') {
      commit(draft);
    }

    skipBlur = false;
    switching = mode.kind !== 'idle';
    mode = { kind: 'edit', tag };
    draft = tag;

    requestAnimationFrame(() => {
      switching = false;
      inputEl?.select();
    });
  };

  const leaveToChip = () => {
    skipBlur = true;
    const index = tags.length - 1;
    close();
    focusChip(index);
  };

  const removeAt = (index: number) => {
    const next = tags.filter((_, i) => i !== index);
    tags = next;

    if (next.length === 0) {
      startAdding();
    } else {
      focusChipLater(index - 1);
    }
  };

  const finishEditing = (index: number) => {
    if (tags.length === 0) {
      startAdding();
      return;
    }

    skipBlur = true;
    close();
    focusChipLater(index);
  };

  const handleBlur = () => {
    if (switching) return;

    if (skipBlur) {
      skipBlur = false;
      return;
    }

    if (mode.kind === 'edit') {
      rename(mode.tag, draft);
      close();
    } else if (mode.kind === 'add') {
      commit(draft);
      close();
    }
  };

  const handleInput = (event: Event) => {
    const value = (event.currentTarget as HTMLInputElement).value;

    if (mode.kind === 'add' && /[,\n\r]/.test(value)) {
      const pieces = value.split(/[,\n\r]/);
      const rest = pieces.pop() ?? '';
      commit(pieces.join(','));
      draft = rest;
      scheduleSuggest(rest);
      return;
    }

    draft = value;
    if (mode.kind === 'add') scheduleSuggest(value);
  };

  const handlePaste = (event: ClipboardEvent) => {
    const text = event.clipboardData?.getData('text') ?? '';
    if (mode.kind !== 'add' || !/[,\n\r]/.test(text)) return;

    event.preventDefault();
    commit(draft + text);
    draft = '';
  };

  const handleKeydown = (event: KeyboardEvent) => {
    if (event.isComposing) return;

    if (mode.kind === 'add' && draft.length === 0 && tags.length > 0 && (event.key === 'Backspace' || event.key === 'ArrowLeft')) {
      event.preventDefault();
      leaveToChip();
      return;
    }

    if (panelOpen && (event.key === 'ArrowDown' || event.key === 'ArrowUp')) {
      event.preventDefault();
      moveHighlight(event.key === 'ArrowDown' ? 1 : -1);
      return;
    }

    if (event.key === 'Enter') {
      event.preventDefault();
      if (mode.kind === 'edit') {
        const index = tags.indexOf(mode.tag);
        const kept = rename(mode.tag, draft);
        finishEditing(kept ? index : index - 1);
      } else {
        const picked = highlighted === null ? undefined : flatSuggestions.at(highlighted);
        if (picked) {
          pickSuggestion(picked.name);
        } else {
          commit(draft);
          draft = '';
          resetSuggest();
        }
      }
    } else if (event.key === 'Escape') {
      event.preventDefault();

      if (mode.kind === 'edit') {
        finishEditing(tags.indexOf(mode.tag));
      } else {
        skipBlur = true;
        close();
      }
    }
  };

  const handleChipKeydown = (event: KeyboardEvent, index: number) => {
    if (event.isComposing) return;

    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      if (index > 0) focusChip(index - 1);
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      if (index < tags.length - 1) {
        focusChip(index + 1);
      } else {
        startAdding();
      }
    } else if (event.key === 'Backspace' || event.key === 'Delete') {
      event.preventDefault();
      removeAt(index);
    }
  };

  const chipFocusStyle = css.raw({
    borderColor: 'accent.default',
    color: 'text.on.inverse',
    backgroundColor: 'surface.inverse',
  });

  const chipIdleStyle = css.raw({
    _hover: { color: 'text.default' },
  });

  const chipFlashStyle = css.raw({
    borderColor: 'accent.default',
    color: 'text.default',
    backgroundColor: 'accent.subtle',
  });

  const chipLockedStyle = css.raw({
    paddingRight: '9px',
  });

  const chipPartialStyle = css.raw({
    borderStyle: 'dashed',
    borderColor: 'border.default',
    color: 'text.hint',
  });

  const inputStyle = css.raw({
    gridArea: '[1 / 1]',
    width: 'full',
    height: '24px',
    paddingX: '10px',
    borderWidth: '1px',
    borderColor: 'accent.default',
    borderRadius: 'full',
    fontSize: '12px',
    fontWeight: 'medium',
    color: 'text.default',
    backgroundColor: 'surface.default',
    _placeholder: { fontWeight: 'medium', color: 'text.hint' },
  });
</script>

{#snippet pillInput(placeholder: string)}
  <span class={css({ display: 'inline-grid', minWidth: '56px', maxWidth: 'full' })} use:suggestAnchor>
    <span
      class={css({
        gridArea: '[1 / 1]',
        height: '0',
        paddingX: '10px',
        overflow: 'hidden',
        fontSize: '12px',
        fontWeight: 'medium',
        whiteSpace: 'pre',
        visibility: 'hidden',
      })}
      aria-hidden="true"
    >
      {`${draft || placeholder} `}
    </span>

    <input
      bind:this={inputEl}
      class={css(inputStyle)}
      aria-activedescendant={highlighted !== null && mode.kind === 'add' ? `${panelId}-${highlighted}` : undefined}
      aria-autocomplete={mode.kind === 'add' ? 'list' : undefined}
      aria-controls={mode.kind === 'add' ? panelId : undefined}
      aria-expanded={mode.kind === 'add' ? panelOpen : undefined}
      aria-label="태그"
      autocomplete="off"
      data-1p-ignore
      onblur={handleBlur}
      oninput={handleInput}
      onkeydown={handleKeydown}
      onpaste={handlePaste}
      {placeholder}
      role={mode.kind === 'add' ? 'combobox' : undefined}
      size="1"
      type="text"
      value={draft}
    />
  </span>
{/snippet}

<div class={flex({ flexWrap: 'wrap', alignItems: 'center', gap: '6px', minHeight: '30px' })}>
  {#each tags as tag, index (tag)}
    {@const editing = mode.kind === 'edit' && mode.tag === tag}

    {#if editing}
      {@render pillInput('')}
    {/if}

    <span
      class={css(
        tagChipStyle,
        disabled ? chipLockedStyle : undefined,
        focusedIndex === index ? chipFocusStyle : chipIdleStyle,
        flashed === tag ? chipFlashStyle : undefined,
      )}
      hidden={editing}
    >
      {#if disabled}
        <span>{tag}</span>
      {:else}
        <button
          bind:this={chipEls[index]}
          class={css({ outlineWidth: '0', color: '[inherit]' })}
          onblur={() => {
            if (focusedIndex === index) focusedIndex = null;
          }}
          onclick={() => startEditing(tag)}
          onfocus={() => (focusedIndex = index)}
          onkeydown={(event) => handleChipKeydown(event, index)}
          onmousedown={(event) => event.preventDefault()}
          type="button"
        >
          {tag}
        </button>

        <button
          class={center({ size: '14px', borderRadius: 'full', color: '[inherit]', opacity: '70', _hover: { opacity: '100' } })}
          aria-label="태그 삭제"
          onclick={() => remove(tag)}
          onmousedown={(event) => event.preventDefault()}
          type="button"
        >
          <Icon icon={XIcon} size={10} />
        </button>
      {/if}
    </span>
  {/each}

  {#each partial.filter((item) => !tags.includes(item.tag)) as item (item.tag)}
    <span
      class={css(tagChipStyle, chipPartialStyle, disabled || !onremovepartial ? chipLockedStyle : undefined)}
      use:tooltip={{ message: `글 ${item.count}개에만`, placement: 'top' }}
    >
      <span>{item.tag}</span>

      {#if !disabled && onremovepartial}
        <button
          class={center({ size: '14px', borderRadius: 'full', color: '[inherit]', opacity: '70', _hover: { opacity: '100' } })}
          aria-label="태그 삭제"
          onclick={() => onremovepartial(item.tag)}
          onmousedown={(event) => event.preventDefault()}
          type="button"
        >
          <Icon icon={XIcon} size={10} />
        </button>
      {/if}
    </span>
  {/each}

  {#if !disabled}
    {#if mode.kind === 'add'}
      {@render pillInput('태그 입력')}
    {/if}

    <button
      bind:this={addButtonEl}
      class={flex({
        alignItems: 'center',
        gap: '3px',
        height: '24px',
        paddingLeft: '9px',
        paddingRight: '7px',
        borderWidth: '1px',
        borderStyle: 'dashed',
        borderColor: 'border.default',
        borderRadius: 'full',
        fontSize: '12px',
        fontWeight: 'medium',
        color: 'text.muted',
        transition: 'common',
        _hover: { borderColor: 'border.emphasis', color: 'text.default', backgroundColor: 'surface.hover' },
      })}
      aria-label="태그 추가"
      data-primary
      hidden={mode.kind === 'add'}
      onclick={startAdding}
      onfocus={startAdding}
      type="button"
    >
      <span>{tags.length > 0 ? '추가' : '태그 추가'}</span>
      <Icon icon={PlusIcon} size={12} />
    </button>
  {:else if tags.length === 0 && partial.length === 0}
    <span class={css({ fontSize: '13px', color: 'text.hint' })}>없음</span>
  {/if}

  <span class={css({ srOnly: true })} aria-live="polite">{announcement}</span>
</div>

{#if panelOpen}
  <div
    bind:this={panelEl}
    class={css({ zIndex: 'tooltip', pointerEvents: 'auto' })}
    use:suggestFloating={{ appendTo: document.querySelector('.tooltip-container') as Element | null }}
  >
    <TagSuggestions
      id={panelId}
      {highlighted}
      mine={visibleSuggestions.mine}
      onselect={pickSuggestion}
      popular={visibleSuggestions.popular}
    />
  </div>
{/if}
