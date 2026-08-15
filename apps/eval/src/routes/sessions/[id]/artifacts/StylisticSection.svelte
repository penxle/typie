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
  import Quote from './Quote.svelte';
  import type { Stylistic } from '$lib/feedback/artifacts.ts';

  // movementTitles는 구획 지도가 서 있을 때의 id→title 사전이다 — 커버 기록의 구획 id 옆에 이름을 병기한다.
  type Props = { value: Stylistic; targets: Set<string>; movementTitles: Map<string, string> };
  const { value, targets, movementTitles }: Props = $props();

  const dash = css({ color: 'text.faint' });
  // 확인 요건은 방법(enum)과 이행 서술이 한 값이다 — 방법 필 뒤에 서술을 잇는다.
  const verificationRow = css({ display: 'inline', '& > span:last-child': { marginLeft: '8px' } });
</script>

<div class={flex({ direction: 'column', gap: '32px' })}>
  <Block>
    <GroupHeading field="stylistic.findings" label="findings" />
    {#if value.findings.length === 0}
      <Empty text="항목이 없어요" />
    {:else}
      <EntryList>
        {#each value.findings as finding, index (index)}
          <Entry id={finding.id ? anchorOf.stylisticFinding(finding.id) : undefined}>
            {#snippet kicker()}
              <IdChip quiet value={finding.id} />
            {/snippet}
            <Quote head={finding.head} tail={finding.tail} />
            <Field field="stylistic.findings.criterion" label="criterion">
              <EnumChip field="stylistic.criterion" value={finding.criterion} />
            </Field>
            <Field field="stylistic.findings.observation" label="observation">{finding.observation}</Field>
            <Field field="stylistic.findings.verification" label="verification">
              <span class={verificationRow}>
                <EnumChip field="verification.method" value={finding.verification.method} />
                <span>{finding.verification.note}</span>
              </span>
            </Field>
            <Field field="stylistic.findings.direction" label="direction">{finding.direction}</Field>
          </Entry>
        {/each}
      </EntryList>
    {/if}
  </Block>

  <Block>
    <GroupHeading field="stylistic.log" label="log" />
    {#if value.log.length === 0}
      <Empty text="처분 항목이 없어요" />
    {:else}
      <DataTable
        columns={[
          { label: 'entry', field: 'stylistic.log.entry' },
          { label: 'disposition', field: 'stylistic.log.disposition' },
          { label: 'finding', field: 'stylistic.log.finding' },
          { label: 'note', field: 'stylistic.log.note' },
        ]}
        rows={value.log}
      >
        {#snippet cell(item)}
          <td><IdChip target={item.entry ? linkTo(targets, anchorOf.experience(item.entry)) : undefined} value={item.entry} /></td>
          <td><EnumChip field="stylistic.log.disposition" value={item.disposition} /></td>
          <td>
            {#if item.finding}
              <IdChip target={linkTo(targets, anchorOf.stylisticFinding(item.finding))} value={item.finding} />
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
    <GroupHeading field="stylistic.coverage" label="coverage" />
    {#if value.coverage.length === 0}
      <Empty text="커버 기록이 없어요" />
    {:else}
      <DataTable
        columns={[
          { label: 'movement', field: 'stylistic.coverage.movement' },
          { label: 'note', field: 'stylistic.coverage.note' },
        ]}
        rows={value.coverage}
      >
        {#snippet cell(item)}
          <td>
            <span class={flex({ align: 'baseline', wrap: 'wrap', gap: '8px' })}>
              <IdChip target={item.movement ? linkTo(targets, anchorOf.movement(item.movement)) : undefined} value={item.movement} />
              {#if movementTitles.has(item.movement)}
                <span class={css({ color: 'text.subtle' })}>{movementTitles.get(item.movement)}</span>
              {/if}
            </span>
          </td>
          <td>{item.note}</td>
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
