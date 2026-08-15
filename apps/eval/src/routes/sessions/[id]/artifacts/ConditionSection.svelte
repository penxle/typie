<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import Block from './Block.svelte';
  import Empty from './Empty.svelte';
  import Entry from './Entry.svelte';
  import EntryList from './EntryList.svelte';
  import EnumChip from './EnumChip.svelte';
  import Field from './Field.svelte';
  import GroupHeading from './GroupHeading.svelte';
  import Quote from './Quote.svelte';
  import type { Condition } from '$lib/feedback/artifacts.ts';

  type Props = { value: Condition };
  const { value }: Props = $props();
</script>

<div class={flex({ direction: 'column', gap: '32px' })}>
  <Block>
    <GroupHeading field="condition.completeness" label="completeness" />
    <div class={flex({ direction: 'column', gap: '6px' })}>
      <Field field="condition.completeness.level" label="level">
        <EnumChip field="condition.completeness.level" value={value.completeness.level} />
      </Field>
      <Field field="condition.completeness.note" label="note">{value.completeness.note}</Field>
    </div>
  </Block>

  <Block>
    <GroupHeading field="condition.exclusions" label="exclusions" />
    {#if value.exclusions.length === 0}
      <Empty text="제외 구간이 없어요" />
    {:else}
      <EntryList>
        {#each value.exclusions as exclusion, index (index)}
          <Entry>
            <Quote head={exclusion.head} tail={exclusion.tail} />
            <Field field="condition.exclusions.reason" label="reason">{exclusion.reason}</Field>
          </Entry>
        {/each}
      </EntryList>
    {/if}
  </Block>
</div>
