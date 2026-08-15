<script lang="ts">
  import { anchorOf } from '$lib/feedback/artifacts.ts';
  import Empty from './Empty.svelte';
  import Entry from './Entry.svelte';
  import EntryList from './EntryList.svelte';
  import EnumChip from './EnumChip.svelte';
  import Field from './Field.svelte';
  import IdChip from './IdChip.svelte';
  import Quote from './Quote.svelte';
  import type { Experience } from '$lib/feedback/artifacts.ts';

  type Props = { value: Experience };
  const { value }: Props = $props();
</script>

{#if value.entries.length === 0}
  <Empty text="기록된 사건이 없어요" />
{:else}
  <EntryList>
    {#each value.entries as entry, index (index)}
      <Entry id={entry.id ? anchorOf.experience(entry.id) : undefined}>
        {#snippet kicker()}
          <IdChip quiet value={entry.id} />
        {/snippet}
        <Quote head={entry.head} tail={entry.tail} />
        <Field field="experience.kind" label="kind"><EnumChip field="experience.kind" value={entry.kind} /></Field>
        <Field field="experience.note" label="note">{entry.note}</Field>
      </Entry>
    {/each}
  </EntryList>
{/if}
