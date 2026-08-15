<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import Block from './Block.svelte';
  import DataTable from './DataTable.svelte';
  import Empty from './Empty.svelte';
  import EnumChip from './EnumChip.svelte';
  import Field from './Field.svelte';
  import GroupHeading from './GroupHeading.svelte';
  import IdChip from './IdChip.svelte';
  import Prose from './Prose.svelte';
  import type { Audience } from '$lib/feedback/artifacts.ts';

  type Props = { value: Audience };
  const { value }: Props = $props();

  const rows = flex({ direction: 'column', gap: '6px' });
  const dash = css({ color: 'text.faint' });
</script>

<div class={flex({ direction: 'column', gap: '32px' })}>
  <Block>
    <GroupHeading field="audience.source" label="source" />
    <div class={rows}>
      <Field field="audience.source.status" label="status"><EnumChip field="audience.source.status" value={value.source.status} /></Field>
      <Field field="audience.source.name" label="name">
        {#if value.source.name}{value.source.name}{:else}<span class={dash}>—</span>{/if}
      </Field>
      <Field field="audience.source.background" label="background">
        {#if value.source.background}{value.source.background}{:else}<span class={dash}>—</span>{/if}
      </Field>
    </div>
  </Block>

  <Block>
    <GroupHeading field="audience.genre" label="genre" />
    <!-- 스칼라 그룹 — 키 없이 값만. 파일에 없는 키를 지어내지 않는다. -->
    {#if value.genre}
      <Prose text={value.genre} />
    {:else}
      <span class={css({ fontSize: '14px', color: 'text.faint' })}>—</span>
    {/if}
  </Block>

  <Block>
    <GroupHeading field="audience.knowledge" label="knowledge" />
    {#if value.knowledge.length === 0}
      <Empty text="항목이 없어요" />
    {:else}
      <DataTable
        columns={[
          { label: 'id', field: 'audience.knowledge.id' },
          { label: 'fact', field: 'audience.knowledge.fact' },
          { label: 'source', field: 'audience.knowledge.source' },
          { label: 'note', field: 'audience.knowledge.note' },
        ]}
        rows={value.knowledge}
      >
        {#snippet cell(item)}
          <td><IdChip value={item.id} /></td>
          <td>{item.fact}</td>
          <td><EnumChip field="audience.knowledge.source" value={item.source} /></td>
          <td>{item.note}</td>
        {/snippet}
      </DataTable>
    {/if}
  </Block>
</div>
