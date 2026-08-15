<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { anchorOf, linkTo } from '$lib/feedback/artifacts.ts';
  import Block from './Block.svelte';
  import DataTable from './DataTable.svelte';
  import Empty from './Empty.svelte';
  import Entry from './Entry.svelte';
  import EntryList from './EntryList.svelte';
  import EnumChip from './EnumChip.svelte';
  import Field from './Field.svelte';
  import GroupHeading from './GroupHeading.svelte';
  import IdChip from './IdChip.svelte';
  import PointMeter from './PointMeter.svelte';
  import Quote from './Quote.svelte';
  import type { Judgment } from '$lib/feedback/artifacts.ts';

  type Props = { value: Judgment; targets: Set<string> };
  const { value, targets }: Props = $props();

  const dash = css({ color: 'text.faint' });
  // 확인 요건은 방법(enum)과 이행 서술이 한 값이다 — 방법 필 뒤에 서술을 잇는다.
  const verificationRow = css({ display: 'inline', '& > span:last-child': { marginLeft: '8px' } });

  // basis는 재검토 회차에만 서는 열이다 — 한 행이라도 실려 있으면 열을 세운다.
  const hasBasis = $derived(value.verdicts.some((verdict) => verdict.basis != null));
  const verdictColumns = $derived([
    { label: 'trait', field: 'judgment.verdicts.trait' as const },
    { label: 'point', field: 'judgment.verdicts.point' as const },
    { label: 'note', field: 'judgment.verdicts.note' as const },
    ...(hasBasis ? [{ label: 'basis', field: 'judgment.verdicts.basis' as const }] : []),
  ]);

  const traitLink = (trait: string) => (trait ? linkTo(targets, anchorOf.trait(trait)) : undefined);
</script>

<div class={flex({ direction: 'column', gap: '32px' })}>
  <Block>
    <GroupHeading field="judgment.verdicts" label="verdicts" />
    {#if value.verdicts.length === 0}
      <Empty text="판정이 없어요" />
    {:else}
      <DataTable columns={verdictColumns} rows={value.verdicts}>
        {#snippet cell(verdict)}
          <td><IdChip target={traitLink(verdict.trait)} value={verdict.trait} /></td>
          <td class={css({ whiteSpace: 'nowrap' })}><PointMeter point={verdict.point} /></td>
          <td>{verdict.note}</td>
          {#if hasBasis}
            <td>
              {#if verdict.basis}<EnumChip field="judgment.verdicts.basis" value={verdict.basis} />{:else}<span class={dash}>—</span>{/if}
            </td>
          {/if}
        {/snippet}
      </DataTable>
    {/if}
  </Block>

  <Block>
    <GroupHeading field="judgment.findings" label="findings" />
    {#if value.findings.length === 0}
      <Empty text="항목이 없어요" />
    {:else}
      <EntryList>
        {#each value.findings as finding, index (index)}
          <Entry id={finding.id ? anchorOf.judgmentFinding(finding.id) : undefined}>
            {#snippet kicker()}
              <IdChip quiet value={finding.id} />
            {/snippet}
            <Quote head={finding.head} tail={finding.tail} />
            <Field field="judgment.findings.trait" label="trait"><IdChip target={traitLink(finding.trait)} value={finding.trait} /></Field>
            <Field field="judgment.findings.condition" label="condition">
              <IdChip
                target={finding.trait && finding.condition
                  ? linkTo(targets, anchorOf.condition(finding.trait, finding.condition))
                  : undefined}
                value={finding.condition}
              />
            </Field>
            <Field field="judgment.findings.observation" label="observation">{finding.observation}</Field>
            <Field field="judgment.findings.verification" label="verification">
              <span class={verificationRow}>
                <EnumChip field="verification.method" value={finding.verification.method} />
                <span>{finding.verification.note}</span>
              </span>
            </Field>
            <Field field="judgment.findings.direction" label="direction">{finding.direction}</Field>
          </Entry>
        {/each}
      </EntryList>
    {/if}
  </Block>

  <Block>
    <GroupHeading field="judgment.elevations" label="elevations" />
    {#if value.elevations.length === 0}
      <Empty text="항목이 없어요" />
    {:else}
      <EntryList>
        {#each value.elevations as elevation, index (index)}
          <Entry>
            {#snippet kicker()}
              <IdChip quiet value={elevation.id} />
            {/snippet}
            <Quote head={elevation.head} tail={elevation.tail} />
            <Field field="judgment.elevations.trait" label="trait">
              <IdChip target={traitLink(elevation.trait)} value={elevation.trait} />
            </Field>
            <Field field="judgment.elevations.observation" label="observation">{elevation.observation}</Field>
            <Field field="judgment.elevations.direction" label="direction">{elevation.direction}</Field>
          </Entry>
        {/each}
      </EntryList>
    {/if}
  </Block>

  <Block>
    <GroupHeading field="judgment.log" label="log" />
    {#if value.log.length === 0}
      <Empty text="처분 항목이 없어요" />
    {:else}
      <DataTable
        columns={[
          { label: 'entry', field: 'judgment.log.entry' },
          { label: 'disposition', field: 'judgment.log.disposition' },
          { label: 'finding', field: 'judgment.log.finding' },
          { label: 'note', field: 'judgment.log.note' },
        ]}
        rows={value.log}
      >
        {#snippet cell(item)}
          <td><IdChip target={item.entry ? linkTo(targets, anchorOf.experience(item.entry)) : undefined} value={item.entry} /></td>
          <td><EnumChip field="judgment.log.disposition" value={item.disposition} /></td>
          <td>
            {#if item.finding}
              <IdChip target={linkTo(targets, anchorOf.judgmentFinding(item.finding))} value={item.finding} />
            {:else}
              <span class={dash}>—</span>
            {/if}
          </td>
          <td>{item.note}</td>
        {/snippet}
      </DataTable>
    {/if}
  </Block>

  <Block>
    <GroupHeading field="judgment.gaps" label="gaps" />
    {#if value.gaps.length === 0}
      <Empty text="신고된 공백이 없어요" />
    {:else}
      <DataTable
        columns={[
          { label: 'id', field: 'judgment.gaps.id' },
          { label: 'note', field: 'judgment.gaps.note' },
        ]}
        rows={value.gaps}
      >
        {#snippet cell(gap)}
          <td><IdChip value={gap.id} /></td>
          <td>{gap.note}</td>
        {/snippet}
      </DataTable>
    {/if}
  </Block>

  {#if value.threads}
    <Block>
      <GroupHeading field="threads" label="threads" />
      {#if value.threads.length === 0}
        <Empty text="처분한 스레드가 없어요" />
      {:else}
        <DataTable
          columns={[
            { label: 'thread', field: 'threads.thread' },
            { label: 'verdict', field: 'threads.verdict' },
            { label: 'note', field: 'threads.note' },
            { label: 'anchor', field: 'threads.anchor' },
          ]}
          rows={value.threads}
        >
          {#snippet cell(item)}
            <td><IdChip value={item.thread} /></td>
            <td><EnumChip field="threads.verdict" value={item.verdict} /></td>
            <td>{item.note}</td>
            <td>
              {#if item.anchor}<Quote head={item.anchor.head} tail={item.anchor.tail} />{:else}<span class={dash}>—</span>{/if}
            </td>
          {/snippet}
        </DataTable>
      {/if}
    </Block>
  {/if}
</div>
