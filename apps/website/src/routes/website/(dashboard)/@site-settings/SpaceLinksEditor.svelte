<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, TextInput } from '@typie/ui/components';
  import PlusIcon from '~icons/lucide/plus';
  import Trash2Icon from '~icons/lucide/trash-2';

  type Link = { label: string; url: string };

  type Props = {
    links: readonly Link[];
    onsave: (links: Link[]) => Promise<boolean>;
  };

  let { links, onsave }: Props = $props();

  let draft = $state<Link[]>(links.map((link) => ({ label: link.label, url: link.url })));
  let saving = $state(false);

  const serialize = (items: readonly Link[]) => JSON.stringify(items.map((link) => ({ label: link.label, url: link.url })));

  const dirty = $derived(serialize(draft) !== serialize(links));

  $effect(() => {
    const next = links.map((link) => ({ label: link.label, url: link.url }));

    if (!dirty) {
      draft = next;
    }
  });

  const save = async () => {
    if (saving || !dirty) return;

    const normalized = draft
      .map((link) => ({ label: link.label.trim(), url: link.url.trim() }))
      .filter((link) => link.label.length > 0 && link.url.length > 0);

    saving = true;
    try {
      if (await onsave(normalized)) {
        draft = normalized.map((link) => ({ ...link }));
      }
    } finally {
      saving = false;
    }
  };
</script>

<div class={flex({ flexDirection: 'column', gap: '8px' })}>
  {#each draft as link, index (index)}
    <div class={flex({ alignItems: 'center', gap: '6px' })}>
      <TextInput style={css.raw({ width: '[140px]', height: '32px', fontSize: '13px' })} placeholder="이름" bind:value={link.label} />
      <TextInput style={css.raw({ flex: '1', height: '32px', fontSize: '13px' })} placeholder="https://" bind:value={link.url} />
      <button
        class={center({
          size: '24px',
          borderRadius: '4px',
          color: 'text.muted',
          _hover: { backgroundColor: 'surface.hover', color: 'danger.default' },
        })}
        onclick={() => (draft = draft.filter((_, i) => i !== index))}
        type="button"
        use:tooltip={{ message: '삭제', placement: 'top' }}
      >
        <Icon icon={Trash2Icon} size={14} />
      </button>
    </div>
  {/each}

  <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
    <Button onclick={() => (draft = [...draft, { label: '', url: '' }])} size="sm" variant="ghost">
      <Icon icon={PlusIcon} size={14} />
      링크 추가
    </Button>
    <Button disabled={!dirty} loading={saving} onclick={save} size="sm" variant="secondary">저장</Button>
  </div>
</div>
