<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { anchorOf } from '$lib/feedback/artifacts.ts';
  import Block from './Block.svelte';
  import Empty from './Empty.svelte';
  import Entry from './Entry.svelte';
  import EntryList from './EntryList.svelte';
  import Field from './Field.svelte';
  import GroupHeading from './GroupHeading.svelte';
  import IdChip from './IdChip.svelte';
  import Prose from './Prose.svelte';
  import Quote from './Quote.svelte';
  import type { Interpretation } from '$lib/feedback/artifacts.ts';

  type Props = { value: Interpretation };
  const { value }: Props = $props();
</script>

<div class={flex({ direction: 'column', gap: '32px' })}>
  <Block>
    <GroupHeading field="interpretation.hypothesis" label="hypothesis" />
    <div class={flex({ direction: 'column', gap: '6px' })}>
      <Field field="interpretation.hypothesis.statement" label="statement">{value.hypothesis.statement}</Field>
      <Field field="interpretation.hypothesis.effect" label="effect">{value.hypothesis.effect}</Field>
    </div>
    <GroupHeading field="interpretation.questions" label="questions" level={2} />
    {#if value.hypothesis.questions.length === 0}
      <Empty text="미결 질문이 없어요" />
    {:else}
      <EntryList>
        {#each value.hypothesis.questions as question, index (index)}
          <Entry id={question.id ? anchorOf.question(question.id) : undefined}>
            {#snippet kicker()}
              <IdChip quiet value={question.id} />
            {/snippet}
            <Prose text={question.question} />
          </Entry>
        {/each}
      </EntryList>
    {/if}
  </Block>

  <Block>
    <GroupHeading field="interpretation.performances" label="performances" />
    {#if value.performances.length === 0}
      <Empty text="등재된 수행이 없어요" />
    {:else}
      <EntryList>
        {#each value.performances as performance, index (index)}
          <Entry id={performance.id ? anchorOf.performance(performance.id) : undefined}>
            {#snippet kicker()}
              <IdChip quiet value={performance.id} />
            {/snippet}
            <Field field="interpretation.performances.evidence" label="evidence">
              <div class={flex({ direction: 'column', gap: '6px' })}>
                {#each performance.evidence as evidence, evidenceIndex (evidenceIndex)}
                  <Quote head={evidence} />
                {/each}
              </div>
            </Field>
            <Field field="interpretation.performances.rationale" label="rationale">{performance.rationale}</Field>
          </Entry>
        {/each}
      </EntryList>
    {/if}
  </Block>

  <Block>
    <GroupHeading field="interpretation.meanings" label="meanings" />
    {#if value.meanings.length === 0}
      <Empty text="항목이 없어요" />
    {:else}
      <EntryList>
        {#each value.meanings as meaning, index (index)}
          <Entry>
            {#snippet kicker()}
              <IdChip quiet value={meaning.id} />
            {/snippet}
            <Quote head={meaning.head} tail={meaning.tail} />
            <Field field="interpretation.meanings.principle" label="principle">{meaning.principle}</Field>
          </Entry>
        {/each}
      </EntryList>
    {/if}
  </Block>
</div>
