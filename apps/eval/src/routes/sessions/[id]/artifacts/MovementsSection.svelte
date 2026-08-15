<script lang="ts">
  import { anchorOf } from '$lib/feedback/artifacts.ts';
  import Empty from './Empty.svelte';
  import Entry from './Entry.svelte';
  import EntryList from './EntryList.svelte';
  import EntryTitle from './EntryTitle.svelte';
  import EnumChip from './EnumChip.svelte';
  import Field from './Field.svelte';
  import IdChip from './IdChip.svelte';
  import Quote from './Quote.svelte';
  import type { Movements } from '$lib/feedback/artifacts.ts';

  type Props = { value: Movements };
  const { value }: Props = $props();
</script>

{#if value.movements.length === 0}
  <Empty text="구획이 없어요" />
{:else}
  <EntryList>
    {#each value.movements as movement, index (index)}
      <Entry id={movement.id ? anchorOf.movement(movement.id) : undefined}>
        {#snippet kicker()}
          <IdChip quiet value={movement.id} />
        {/snippet}
        <EntryTitle text={movement.title} />
        <Quote head={movement.head} tail={movement.tail} />
        <Field field="movements.mode" label="mode"><EnumChip field="movements.mode" value={movement.mode} /></Field>
        <Field field="movements.basis" label="basis">{movement.basis}</Field>
        <Field field="movements.says" label="says">{movement.says}</Field>
        <Field field="movements.does" label="does">{movement.does}</Field>
      </Entry>
    {/each}
  </EntryList>
{/if}
