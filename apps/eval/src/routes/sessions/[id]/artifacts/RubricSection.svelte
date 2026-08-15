<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { anchorOf, linkTo } from '$lib/feedback/artifacts.ts';
  import Block from './Block.svelte';
  import DataTable from './DataTable.svelte';
  import Empty from './Empty.svelte';
  import Entry from './Entry.svelte';
  import EntryList from './EntryList.svelte';
  import EntryTitle from './EntryTitle.svelte';
  import EnumChip from './EnumChip.svelte';
  import Field from './Field.svelte';
  import GroupHeading from './GroupHeading.svelte';
  import IdChip from './IdChip.svelte';
  import PointMeter from './PointMeter.svelte';
  import type { Rubric } from '$lib/feedback/artifacts.ts';

  type Props = { value: Rubric; targets: Set<string> };
  const { value, targets }: Props = $props();

  const bullets = css({ paddingLeft: '18px', listStyleType: 'disc', '& li': { marginY: '3px' } });
  const dash = css({ color: 'text.faint' });

  // 처분 대상은 from이 가리키는 목록의 id다 — 해석 산출물이 서 있을 때만 링크가 된다.
  const subjectTarget = (from: string, subject: string): string | undefined => {
    if (!subject) return undefined;
    if (from === 'performance') return linkTo(targets, anchorOf.performance(subject));
    if (from === 'question') return linkTo(targets, anchorOf.question(subject));
    return undefined;
  };
</script>

<div class={flex({ direction: 'column', gap: '32px' })}>
  <Block>
    <GroupHeading field="rubric.traits" label="traits" />
    {#if value.traits.length === 0}
      <Empty text="특질이 없어요" />
    {:else}
      <EntryList>
        {#each value.traits as trait, index (index)}
          <Entry id={trait.id ? anchorOf.trait(trait.id) : undefined}>
            <!-- 특질 id는 이 특질의 이름이자 판정이 참조하는 키다 — 식별자라 mono지만 리드의 무게로 세운다. -->
            <EntryTitle mono text={trait.id} />
            <Field field="rubric.traits.rationale" label="rationale">{trait.rationale}</Field>
            <Field field="rubric.traits.findings" label="findings" layout="block">
              {#if trait.guide.findings.length === 0}
                <span class={dash}>—</span>
              {:else}
                <DataTable
                  columns={[
                    { label: 'id', field: 'rubric.traits.findings.id' },
                    { label: 'condition', field: 'rubric.traits.findings.condition' },
                  ]}
                  rowId={(finding) => (trait.id && finding.id ? anchorOf.condition(trait.id, finding.id) : undefined)}
                  rows={trait.guide.findings}
                >
                  {#snippet cell(finding)}
                    <td><IdChip value={finding.id} /></td>
                    <td>{finding.condition}</td>
                  {/snippet}
                </DataTable>
              {/if}
            </Field>
            <Field field="rubric.traits.waivers" label="waivers" layout="block">
              {#if trait.guide.waivers.length === 0}
                <span class={dash}>—</span>
              {:else}
                <ul class={bullets}>
                  {#each trait.guide.waivers as waiver, waiverIndex (waiverIndex)}
                    <li>{waiver}</li>
                  {/each}
                </ul>
              {/if}
            </Field>
            <Field field="rubric.traits.scores" label="scores" layout="block">
              {#if trait.guide.scores.length === 0}
                <span class={dash}>—</span>
              {:else}
                <DataTable
                  columns={[
                    { label: 'point', field: 'rubric.traits.scores.point' },
                    { label: 'condition', field: 'rubric.traits.scores.condition' },
                  ]}
                  rows={trait.guide.scores.toSorted((a, b) => a.point - b.point)}
                >
                  {#snippet cell(score)}
                    <td class={css({ whiteSpace: 'nowrap' })}><PointMeter point={score.point} /></td>
                    <td>{score.condition}</td>
                  {/snippet}
                </DataTable>
              {/if}
            </Field>
            <Field field="rubric.traits.edges" label="edges" layout="block">
              {#if trait.guide.edges.length === 0}
                <span class={dash}>—</span>
              {:else}
                <ul class={bullets}>
                  {#each trait.guide.edges as edge, edgeIndex (edgeIndex)}
                    <li>{edge}</li>
                  {/each}
                </ul>
              {/if}
            </Field>
            <Field field="rubric.traits.verification" label="verification" layout="block">{trait.guide.verification}</Field>
          </Entry>
        {/each}
      </EntryList>
    {/if}
  </Block>

  <Block>
    <GroupHeading field="rubric.coverage" label="coverage" />
    {#if value.coverage.length === 0}
      <Empty text="처분 항목이 없어요" />
    {:else}
      <DataTable
        columns={[
          { label: 'subject', field: 'rubric.coverage.subject' },
          { label: 'from', field: 'rubric.coverage.from' },
          { label: 'disposition', field: 'rubric.coverage.disposition' },
          { label: 'trait', field: 'rubric.coverage.trait' },
          { label: 'note', field: 'rubric.coverage.note' },
        ]}
        rows={value.coverage}
      >
        {#snippet cell(item)}
          <td><IdChip target={subjectTarget(item.from, item.subject)} value={item.subject} /></td>
          <td><EnumChip field="rubric.coverage.from" value={item.from} /></td>
          <td><EnumChip field="rubric.coverage.disposition" value={item.disposition} /></td>
          <td>
            {#if item.trait}
              <IdChip target={linkTo(targets, anchorOf.trait(item.trait))} value={item.trait} />
            {:else}
              <span class={dash}>—</span>
            {/if}
          </td>
          <td>{item.note}</td>
        {/snippet}
      </DataTable>
    {/if}
  </Block>
</div>
