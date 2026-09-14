<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, TextInput } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import PencilIcon from '~icons/lucide/pencil';
  import PlusIcon from '~icons/lucide/plus';
  import TrashIcon from '~icons/lucide/trash';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';

  type Link = { label: string; url: string };

  type Props = {
    links: readonly Link[];
    onsave: (links: Link[]) => Promise<boolean>;
  };

  let { links, onsave }: Props = $props();

  let creatingNew = $state(false);
  let editingIndex = $state<number | null>(null);
  let formLabel = $state('');
  let formUrl = $state('');
  let formError = $state('');
  let saving = $state(false);

  const resetForm = () => {
    formLabel = '';
    formUrl = '';
    formError = '';
  };

  const startCreate = () => {
    editingIndex = null;
    creatingNew = true;
    resetForm();
  };

  const startEdit = (index: number) => {
    creatingNew = false;
    editingIndex = index;
    formLabel = links[index].label;
    formUrl = links[index].url;
    formError = '';
  };

  const cancelForm = () => {
    creatingNew = false;
    editingIndex = null;
    resetForm();
  };

  const isHttpUrl = (url: string) => {
    try {
      const { protocol } = new URL(url);
      return protocol === 'http:' || protocol === 'https:';
    } catch {
      return false;
    }
  };

  const validateForm = (): boolean => {
    if (!formLabel.trim()) {
      formError = '링크 이름을 입력해 주세요.';
      return false;
    }
    if (!isHttpUrl(formUrl.trim())) {
      formError = '링크 주소는 http 또는 https로 시작해야 해요.';
      return false;
    }
    formError = '';
    return true;
  };

  const handleSave = async () => {
    if (saving || !validateForm()) return;

    const item = { label: formLabel.trim(), url: formUrl.trim() };
    const next = creatingNew ? [...links, item] : links.map((link, index) => (index === editingIndex ? item : link));

    saving = true;
    try {
      if (await onsave(next)) {
        cancelForm();
      }
    } finally {
      saving = false;
    }
  };

  const handleDelete = (index: number) => {
    Dialog.confirm({
      title: '링크 삭제',
      message: `"${links[index].label}" 링크를 삭제하시겠어요?`,
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        await onsave(links.filter((_, i) => i !== index));
      },
    });
  };
</script>

{#snippet form()}
  <div class={css({ paddingX: '20px', paddingY: '16px' })}>
    <div class={flex({ alignItems: 'center', gap: '8px', marginBottom: '12px' })}>
      <TextInput style={css.raw({ width: '[160px]' })} autofocus placeholder="링크 이름" size="sm" bind:value={formLabel} />
      <TextInput style={css.raw({ flex: '1' })} placeholder="https://example.com" size="sm" bind:value={formUrl} />
    </div>
    <div class={flex({ justifyContent: 'flex-end', gap: '8px' })}>
      <Button onclick={cancelForm} size="sm" variant="secondary">취소</Button>
      <Button loading={saving} onclick={handleSave} size="sm" variant="primary">저장</Button>
    </div>
    {#if formError}
      <p class={css({ fontSize: '12px', color: 'danger.default', marginTop: '8px' })}>{formError}</p>
    {/if}
  </div>
{/snippet}

<div>
  <div class={flex({ alignItems: 'center', justifyContent: 'space-between', marginBottom: '4px' })}>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default' })}>링크</h2>
    <button
      class={flex({
        alignItems: 'center',
        gap: '6px',
        borderRadius: '6px',
        paddingX: '12px',
        paddingY: '6px',
        fontSize: '13px',
        fontWeight: 'medium',
        color: 'text.muted',
        transition: 'common',
        _hover: { backgroundColor: 'surface.hover' },
      })}
      onclick={() => startCreate()}
      type="button"
    >
      <Icon style={css.raw({ color: 'text.muted' })} icon={PlusIcon} size={14} />
      <span>추가</span>
    </button>
  </div>
  <p class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]', marginBottom: '20px' })}>
    소개 아래에 보이는 외부 링크예요. SNS나 다른 사이트 주소를 넣을 수 있어요.
  </p>

  {#if links.length > 0}
    <SettingsCard>
      {#each links as link, index (index)}
        {#if index > 0}
          <SettingsDivider />
        {/if}
        {#if editingIndex === index}
          {@render form()}
        {:else}
          <SettingsRow>
            {#snippet label()}
              {link.label}
            {/snippet}
            {#snippet description()}
              <span class={css({ wordBreak: 'break-all' })}>{link.url}</span>
            {/snippet}
            {#snippet value()}
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <button
                  class={css({
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    size: '28px',
                    borderRadius: '6px',
                    color: 'text.muted',
                    transition: 'common',
                    _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
                  })}
                  onclick={() => startEdit(index)}
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
                    color: 'text.muted',
                    transition: 'common',
                    _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
                  })}
                  onclick={() => handleDelete(index)}
                  type="button"
                >
                  <Icon icon={TrashIcon} size={14} />
                </button>
              </div>
            {/snippet}
          </SettingsRow>
        {/if}
      {/each}
    </SettingsCard>
  {:else if !creatingNew}
    <SettingsCard>
      <div class={css({ padding: '20px', fontSize: '13px', color: 'text.hint', textAlign: 'center' })}>아직 추가한 링크가 없어요.</div>
    </SettingsCard>
  {/if}

  {#if creatingNew}
    <div class={css({ marginTop: links.length > 0 ? '12px' : '0' })}>
      <SettingsCard>
        {@render form()}
      </SettingsCard>
    </div>
  {/if}
</div>
