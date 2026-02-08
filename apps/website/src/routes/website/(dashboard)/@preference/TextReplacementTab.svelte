<script lang="ts">
  import { validateRegex } from '@typie/editor';
  import { cache } from '@typie/sark/internal';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Switch, TextInput, Tooltip } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { animateFlip, createDndHandler } from '@typie/ui/utils';
  import GripVerticalIcon from '~icons/lucide/grip-vertical';
  import InfoIcon from '~icons/lucide/info';
  import PencilIcon from '~icons/lucide/pencil';
  import PlusIcon from '~icons/lucide/plus';
  import TrashIcon from '~icons/lucide/trash';
  import { fragment, graphql } from '$graphql';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import type { DashboardLayout_PreferenceModal_TextReplacementTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_TextReplacementTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_TextReplacementTab_user on User {
        id
        textReplacements {
          ... on TextReplacement {
            id
            match
            substitute
            regex
            preset
            note
            order
          }
          ... on TextReplacementPreference {
            id
            state
            order
            textReplacement {
              id
              match
              substitute
              regex
              preset
              note
              order
            }
          }
        }
      }
    `),
  );

  const createTextReplacement = graphql(`
    mutation DashboardLayout_PreferenceModal_TextReplacementTab_CreateTextReplacement_Mutation($input: CreateTextReplacementInput!) {
      createTextReplacement(input: $input) {
        ... on TextReplacement {
          id
        }
        ... on TextReplacementPreference {
          id
        }
      }
    }
  `);

  const updateTextReplacement = graphql(`
    mutation DashboardLayout_PreferenceModal_TextReplacementTab_UpdateTextReplacement_Mutation($input: UpdateTextReplacementInput!) {
      updateTextReplacement(input: $input) {
        ... on TextReplacement {
          id
        }
        ... on TextReplacementPreference {
          id
        }
      }
    }
  `);

  const deleteTextReplacement = graphql(`
    mutation DashboardLayout_PreferenceModal_TextReplacementTab_DeleteTextReplacement_Mutation($input: DeleteTextReplacementInput!) {
      deleteTextReplacement(input: $input) {
        ... on TextReplacement {
          id
        }
        ... on TextReplacementPreference {
          id
        }
      }
    }
  `);

  const moveTextReplacement = graphql(`
    mutation DashboardLayout_PreferenceModal_TextReplacementTab_MoveTextReplacement_Mutation($input: MoveTextReplacementInput!) {
      moveTextReplacement(input: $input) {
        ... on TextReplacement {
          id
        }
        ... on TextReplacementPreference {
          id
        }
      }
    }
  `);

  type NormalizedItem = {
    textReplacementId: string;
    preferenceId: string | null;
    match: string;
    substitute: string;
    regex: boolean;
    preset: boolean;
    state: 'ACTIVE' | 'DISABLED';
    order: string | null;
    note: string | null;
  };

  const normalize = (item: (typeof $user.textReplacements)[number]): NormalizedItem => {
    if ('textReplacement' in item) {
      return {
        textReplacementId: item.textReplacement.id,
        preferenceId: item.id,
        match: item.textReplacement.match,
        substitute: item.textReplacement.substitute,
        regex: item.textReplacement.regex,
        preset: item.textReplacement.preset,
        state: item.state as 'ACTIVE' | 'DISABLED',
        order: item.order ?? item.textReplacement.order ?? null,
        note: item.textReplacement.note ?? null,
      };
    }
    return {
      textReplacementId: item.id,
      preferenceId: null,
      match: item.match,
      substitute: item.substitute,
      regex: item.regex,
      preset: item.preset,
      state: 'ACTIVE',
      order: item.order ?? null,
      note: item.note ?? null,
    };
  };

  // spell-checker:disable
  const smartQuoteIds = new Set(['TXR0SQUOTEOPEN', 'TXR0SQUOTECLOSE', 'TXR0DQUOTEOPEN', 'TXR0DQUOTECLOSE']);
  // spell-checker:enable

  const items = $derived($user.textReplacements.map(normalize));
  const allPresets = $derived(items.filter((item) => item.preset).toSorted((a, b) => (a.order ?? '').localeCompare(b.order ?? '')));
  const smartQuoteItems = $derived(allPresets.filter((item) => smartQuoteIds.has(item.textReplacementId)));
  const presets = $derived(allPresets.filter((item) => !smartQuoteIds.has(item.textReplacementId)));
  const smartQuoteAllActive = $derived(smartQuoteItems.every((item) => item.state === 'ACTIVE'));
  const customItems = $derived(items.filter((item) => !item.preset).toSorted((a, b) => (a.order ?? '').localeCompare(b.order ?? '')));
  let optimisticOrder = $state<string[] | null>(null);
  $effect(() => {
    void customItems;
    optimisticOrder = null;
  });
  const displayItems = $derived.by(() => {
    if (draggingItemId && dropIndex !== null) {
      const without = customItems.filter((i) => i.textReplacementId !== draggingItemId);
      const dragged = customItems.find((i) => i.textReplacementId === draggingItemId);
      if (!dragged) return customItems;
      const target = Math.min(dropIndex, without.length);
      return [...without.slice(0, target), dragged, ...without.slice(target)];
    }
    if (optimisticOrder) {
      const map = new Map(customItems.map((i) => [i.textReplacementId, i]));
      return optimisticOrder.map((id) => map.get(id)).filter((i): i is NormalizedItem => !!i);
    }
    return customItems;
  });

  const invalidateCache = () => {
    cache.invalidate({ __typename: 'User', id: $user.id, field: 'textReplacements' });
  };

  const toggleState = async (item: NormalizedItem) => {
    const newState = item.state === 'ACTIVE' ? 'DISABLED' : 'ACTIVE';
    await updateTextReplacement({ textReplacementId: item.textReplacementId, state: newState });
    invalidateCache();
  };

  const toggleSmartQuotes = async () => {
    const newState = smartQuoteAllActive ? 'DISABLED' : 'ACTIVE';
    await Promise.all(smartQuoteItems.map((item) => updateTextReplacement({ textReplacementId: item.textReplacementId, state: newState })));
    invalidateCache();
  };

  let creatingNew = $state(false);
  let editingId = $state<string | null>(null);

  let formMatch = $state('');
  let formSubstitute = $state('');
  let formRegex = $state(false);
  let formNote = $state('');
  let formError = $state('');

  const resetForm = () => {
    formMatch = '';
    formSubstitute = '';
    formRegex = false;
    formNote = '';
    formError = '';
  };

  const startCreate = () => {
    editingId = null;
    creatingNew = true;
    resetForm();
  };

  const startEdit = (item: NormalizedItem) => {
    creatingNew = false;
    editingId = item.textReplacementId;
    formMatch = item.match;
    formSubstitute = item.substitute;
    formRegex = item.regex;
    formNote = item.note ?? '';
    formError = '';
  };

  const cancelForm = () => {
    creatingNew = false;
    editingId = null;
    resetForm();
  };

  const validateForm = (): boolean => {
    if (!formMatch.trim()) {
      formError = '찾을 텍스트를 입력해 주세요.';
      return false;
    }
    if (!formSubstitute.trim()) {
      formError = '삽입할 텍스트를 입력해 주세요.';
      return false;
    }
    if (formMatch === formSubstitute) {
      formError = '찾을 텍스트와 삽입할 텍스트가 같아요.';
      return false;
    }
    if (formRegex && !validateRegex(formMatch)) {
      formError = '유효하지 않은 정규식이에요.';
      return false;
    }
    formError = '';
    return true;
  };

  const handleSave = async () => {
    if (!validateForm()) return;

    if (creatingNew) {
      const lastOrder = customItems.at(-1)?.order ?? undefined;
      await createTextReplacement({
        match: formMatch,
        substitute: formSubstitute,
        regex: formRegex,
        note: formNote,
        lowerOrder: lastOrder,
      });
    } else if (editingId) {
      await updateTextReplacement({
        textReplacementId: editingId,
        match: formMatch,
        substitute: formSubstitute,
        regex: formRegex,
        note: formNote,
      });
    }

    invalidateCache();
    cancelForm();
  };

  const handleDelete = (item: NormalizedItem) => {
    Dialog.confirm({
      title: '대치 규칙 삭제',
      message: `"${item.match} → ${item.substitute}" 규칙을 삭제하시겠어요?`,
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        await deleteTextReplacement({ textReplacementId: item.textReplacementId });
        invalidateCache();
      },
    });
  };

  let customListElement = $state<HTMLDivElement>();
  let draggingItemId = $state<string | null>(null);
  let dropIndex = $state<number | null>(null);
  let dragStartRects: { top: number; height: number }[] = [];

  const captureDragRects = () => {
    if (!customListElement) return;
    const elements = [...customListElement.querySelectorAll('[data-item-id]')] as HTMLElement[];
    dragStartRects = elements.map((el) => {
      const rect = el.getBoundingClientRect();
      return { top: rect.top, height: rect.height };
    });
  };

  const getDropIndex = (e: PointerEvent) => {
    for (const [i, rect] of dragStartRects.entries()) {
      if (e.clientY < rect.top + rect.height / 2) {
        return i;
      }
    }
    return dragStartRects.length;
  };

  $effect(() => {
    if (!customListElement) return;

    const dndHandler = createDndHandler(customListElement, {
      dragHandleSelector: '[data-drag-handle]',
      excludeSelectors: [],
      getDragTarget: (e) => {
        const target = e.target as HTMLElement;
        if (!target.closest('[data-drag-handle]')) return null;
        return target.closest('[data-item-id]') as HTMLElement;
      },
      canStartDrag: (e, element) => {
        const itemId = element.dataset.itemId;
        if (!itemId) return false;
        e.preventDefault();
        return true;
      },
      onDragStart: (_e, element) => {
        captureDragRects();
        draggingItemId = element.dataset.itemId ?? null;
      },
      onDragMove: (e) => {
        dropIndex = getDropIndex(e);
      },
      onDragEnd: async () => {
        if (!draggingItemId || dropIndex === null) {
          draggingItemId = null;
          dropIndex = null;
          return;
        }

        const currentItemId = draggingItemId;
        const sorted = customItems.filter((i) => i.textReplacementId !== currentItemId);

        const targetIndex = Math.min(dropIndex, sorted.length);
        const lowerOrder = targetIndex > 0 ? (sorted[targetIndex - 1]?.order ?? undefined) : undefined;
        const upperOrder = targetIndex < sorted.length ? (sorted[targetIndex]?.order ?? undefined) : undefined;

        optimisticOrder = displayItems.map((i) => i.textReplacementId);
        draggingItemId = null;
        dropIndex = null;

        await moveTextReplacement({ textReplacementId: currentItemId, lowerOrder, upperOrder });
        invalidateCache();
      },
      onDragCancel: () => {
        draggingItemId = null;
        dropIndex = null;
      },
    });

    return () => {
      dndHandler.destroy();
    };
  });

  $effect.pre(() => {
    void displayItems;
    if (!customListElement) return;
    animateFlip('[data-flip-id]', 'flipId', customListElement);
  });
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default', marginBottom: '4px' })}>텍스트 대치</h1>
    <p class={css({ fontSize: '13px', color: 'text.subtle', lineHeight: '[1.6]' })}>
      입력 중 특정 텍스트를 자동으로 변환해요. v2 에디터에서만 적용돼요.
    </p>
  </div>

  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '20px' })}>기본 대치</h2>

    <SettingsCard>
      {#each presets as item, index (item.textReplacementId)}
        {#if index > 0}
          <SettingsDivider />
        {/if}
        <SettingsRow>
          {#snippet label()}
            {#if item.note}
              <div class={flex({ alignItems: 'center', gap: '6px' })}>
                <span>{item.note}</span>
                {#if item.regex}
                  <span
                    class={css({
                      fontSize: '11px',
                      fontWeight: 'medium',
                      color: 'text.faint',
                      borderWidth: '1px',
                      borderColor: 'border.subtle',
                      paddingX: '6px',
                      paddingY: '2px',
                      borderRadius: 'full',
                    })}
                  >
                    정규식
                  </span>
                {/if}
                {#snippet tooltipMessage()}
                  <span class={css({ fontFamily: 'mono' })}>
                    {item.match}
                    <span class={css({ marginX: '4px', color: 'text.faint' })}>→</span>
                    {item.substitute}
                  </span>
                {/snippet}
                <Tooltip message={tooltipMessage} placement="top">
                  <Icon style={css.raw({ color: 'text.disabled' })} icon={InfoIcon} size={14} />
                </Tooltip>
              </div>
            {:else}
              <span class={flex({ alignItems: 'center', gap: '6px', fontFamily: 'mono', fontSize: '12px' })}>
                <code class={css({ backgroundColor: 'surface.muted', paddingX: '6px', paddingY: '2px', borderRadius: '4px' })}>
                  {item.match}
                </code>
                <span class={css({ color: 'text.faint' })}>→</span>
                <code class={css({ backgroundColor: 'surface.muted', paddingX: '6px', paddingY: '2px', borderRadius: '4px' })}>
                  {item.substitute}
                </code>
              </span>
              {#if item.regex}
                <span
                  class={css({
                    marginLeft: '8px',
                    fontSize: '11px',
                    fontWeight: 'medium',
                    color: 'text.faint',
                    borderWidth: '1px',
                    borderColor: 'border.subtle',
                    paddingX: '6px',
                    paddingY: '2px',
                    borderRadius: 'full',
                  })}
                >
                  정규식
                </span>
              {/if}
            {/if}
          {/snippet}
          {#snippet value()}
            <Switch checked={item.state === 'ACTIVE'} onchange={() => toggleState(item)} />
          {/snippet}
        </SettingsRow>
      {/each}
      {#if smartQuoteItems.length > 0}
        {#if presets.length > 0}
          <SettingsDivider />
        {/if}
        <SettingsRow>
          {#snippet label()}
            <div class={flex({ alignItems: 'center', gap: '6px' })}>
              <span>곧은따옴표를 둥근따옴표로</span>
              {#snippet smartQuoteTooltip()}
                <div class={flex({ direction: 'column', gap: '4px', fontFamily: 'mono' })}>
                  {#each smartQuoteItems as sq (sq.textReplacementId)}
                    <span>
                      {sq.match}
                      <span class={css({ marginX: '4px', color: 'text.faint' })}>→</span>
                      {sq.substitute}
                    </span>
                  {/each}
                </div>
              {/snippet}
              <Tooltip message={smartQuoteTooltip} placement="top">
                <Icon style={css.raw({ color: 'text.disabled' })} icon={InfoIcon} size={14} />
              </Tooltip>
            </div>
          {/snippet}
          {#snippet value()}
            <Switch checked={smartQuoteAllActive} onchange={() => toggleSmartQuotes()} />
          {/snippet}
        </SettingsRow>
      {/if}
    </SettingsCard>
  </div>

  <div>
    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', marginBottom: '4px' })}>
      <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default' })}>사용자 대치</h2>
      <button
        class={flex({
          alignItems: 'center',
          gap: '6px',
          borderRadius: '6px',
          paddingX: '12px',
          paddingY: '6px',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.subtle',
          transition: 'common',
          _hover: { backgroundColor: 'surface.muted' },
        })}
        onclick={() => startCreate()}
        type="button"
      >
        <Icon style={css.raw({ color: 'text.faint' })} icon={PlusIcon} size={14} />
        <span>추가</span>
      </button>
    </div>
    <p class={css({ fontSize: '13px', color: 'text.subtle', lineHeight: '[1.6]', marginBottom: '20px' })}>
      위에서부터 순서대로 먼저 매치되는 규칙이 적용돼요.
    </p>

    {#if displayItems.length > 0}
      <SettingsCard>
        <div bind:this={customListElement}>
          {#each displayItems as item, index (item.textReplacementId)}
            {#if index > 0}
              <SettingsDivider />
            {/if}
            {#if editingId === item.textReplacementId}
              <div class={css({ paddingX: '20px', paddingY: '16px' })}>
                <div class={flex({ alignItems: 'center', gap: '8px', marginBottom: '12px' })}>
                  <TextInput
                    style={css.raw({ flex: '1', fontFamily: 'mono' })}
                    placeholder="찾을 텍스트"
                    size="sm"
                    bind:value={formMatch}
                  />
                  <span class={css({ color: 'text.faint', fontSize: '13px' })}>→</span>
                  <TextInput
                    style={css.raw({ flex: '1', fontFamily: 'mono' })}
                    placeholder="삽입할 텍스트"
                    size="sm"
                    bind:value={formSubstitute}
                  />
                </div>
                <div class={css({ marginBottom: '12px' })}>
                  <TextInput style={css.raw({ width: 'full' })} placeholder="설명 (선택)" size="sm" bind:value={formNote} />
                </div>
                <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
                  <div class={flex({ alignItems: 'center', gap: '8px' })}>
                    <Switch checked={formRegex} onchange={() => (formRegex = !formRegex)} />
                    <span class={css({ fontSize: '13px', color: 'text.subtle' })}>정규식</span>
                  </div>
                  <div class={flex({ gap: '8px' })}>
                    <Button onclick={cancelForm} size="sm" variant="secondary">취소</Button>
                    <Button onclick={handleSave} size="sm" variant="primary">저장</Button>
                  </div>
                </div>
                {#if formError}
                  <p class={css({ fontSize: '12px', color: 'text.danger', marginTop: '8px' })}>{formError}</p>
                {/if}
              </div>
            {:else}
              <div data-flip-id={item.textReplacementId} data-item-id={item.textReplacementId}>
                <SettingsRow>
                  {#snippet label()}
                    <div class={flex({ alignItems: 'center', gap: '8px' })}>
                      <div
                        class={css({
                          cursor: 'grab',
                          color: 'text.faint',
                          _hover: { color: 'text.subtle' },
                          _active: { cursor: 'grabbing' },
                        })}
                        data-drag-handle
                      >
                        <Icon icon={GripVerticalIcon} size={16} />
                      </div>
                      <span
                        class={css({
                          fontSize: '11px',
                          color: 'text.faint',
                          fontVariantNumeric: 'tabular-nums',
                          backgroundColor: 'surface.muted',
                          paddingX: '6px',
                          paddingY: '2px',
                          borderRadius: '4px',
                        })}
                      >
                        {index + 1}
                      </span>
                      {#if item.note}
                        <span>{item.note}</span>
                        {#if item.regex}
                          <span
                            class={css({
                              fontSize: '11px',
                              fontWeight: 'medium',
                              color: 'text.faint',
                              borderWidth: '1px',
                              borderColor: 'border.subtle',
                              paddingX: '6px',
                              paddingY: '2px',
                              borderRadius: 'full',
                            })}
                          >
                            정규식
                          </span>
                        {/if}
                        {#snippet customTooltipMessage()}
                          <span class={css({ fontFamily: 'mono' })}>
                            {item.match}
                            <span class={css({ marginX: '4px', color: 'text.faint' })}>→</span>
                            {item.substitute}
                          </span>
                        {/snippet}
                        <Tooltip message={customTooltipMessage} placement="top">
                          <Icon style={css.raw({ color: 'text.disabled' })} icon={InfoIcon} size={14} />
                        </Tooltip>
                      {:else}
                        <span class={flex({ alignItems: 'center', gap: '6px', fontFamily: 'mono', fontSize: '12px' })}>
                          <code class={css({ backgroundColor: 'surface.muted', paddingX: '6px', paddingY: '2px', borderRadius: '4px' })}>
                            {item.match}
                          </code>
                          <span class={css({ color: 'text.faint' })}>→</span>
                          <code class={css({ backgroundColor: 'surface.muted', paddingX: '6px', paddingY: '2px', borderRadius: '4px' })}>
                            {item.substitute}
                          </code>
                        </span>
                        {#if item.regex}
                          <span
                            class={css({
                              fontSize: '11px',
                              fontWeight: 'medium',
                              color: 'text.faint',
                              borderWidth: '1px',
                              borderColor: 'border.subtle',
                              paddingX: '6px',
                              paddingY: '2px',
                              borderRadius: 'full',
                            })}
                          >
                            정규식
                          </span>
                        {/if}
                      {/if}
                    </div>
                  {/snippet}
                  {#snippet value()}
                    <div class={flex({ alignItems: 'center', gap: '8px' })}>
                      <Switch checked={item.state === 'ACTIVE'} onchange={() => toggleState(item)} />
                      <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.subtle' })}></div>
                      <button
                        class={css({
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center',
                          size: '28px',
                          borderRadius: '6px',
                          color: 'text.faint',
                          transition: 'common',
                          _hover: { backgroundColor: 'surface.muted', color: 'text.subtle' },
                        })}
                        onclick={() => startEdit(item)}
                        type="button"
                      >
                        <Icon icon={PencilIcon} size={14} />
                      </button>
                      <button
                        class={css({
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center',
                          size: '28px',
                          borderRadius: '6px',
                          color: 'text.faint',
                          transition: 'common',
                          _hover: { backgroundColor: 'surface.muted', color: 'text.subtle' },
                        })}
                        onclick={() => handleDelete(item)}
                        type="button"
                      >
                        <Icon icon={TrashIcon} size={14} />
                      </button>
                    </div>
                  {/snippet}
                </SettingsRow>
              </div>
            {/if}
          {/each}
        </div>
      </SettingsCard>
    {:else if !creatingNew}
      <SettingsCard>
        <div class={css({ padding: '20px', fontSize: '13px', color: 'text.subtle', textAlign: 'center' })}>
          아직 사용자 대치 규칙이 없어요.
        </div>
      </SettingsCard>
    {/if}

    {#if creatingNew}
      <div class={css({ marginTop: displayItems.length > 0 ? '12px' : '0' })}>
        <SettingsCard>
          <div class={css({ paddingX: '20px', paddingY: '16px' })}>
            <div class={flex({ alignItems: 'center', gap: '8px', marginBottom: '12px' })}>
              <TextInput
                style={css.raw({ flex: '1', fontFamily: 'mono' })}
                autofocus
                placeholder="찾을 텍스트"
                size="sm"
                bind:value={formMatch}
              />
              <span class={css({ color: 'text.faint', fontSize: '13px' })}>→</span>
              <TextInput
                style={css.raw({ flex: '1', fontFamily: 'mono' })}
                placeholder="삽입할 텍스트"
                size="sm"
                bind:value={formSubstitute}
              />
            </div>
            <div class={css({ marginBottom: '12px' })}>
              <TextInput style={css.raw({ width: 'full' })} placeholder="설명 (선택)" size="sm" bind:value={formNote} />
            </div>
            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Switch checked={formRegex} onchange={() => (formRegex = !formRegex)} />
                <span class={css({ fontSize: '13px', color: 'text.subtle' })}>정규식</span>
              </div>
              <div class={flex({ gap: '8px' })}>
                <Button onclick={cancelForm} size="sm" variant="secondary">취소</Button>
                <Button onclick={handleSave} size="sm" variant="primary">저장</Button>
              </div>
            </div>
            {#if formError}
              <p class={css({ fontSize: '12px', color: 'text.danger', marginTop: '8px' })}>{formError}</p>
            {/if}
          </div>
        </SettingsCard>
      </div>
    {/if}
  </div>
</div>
