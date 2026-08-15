<script lang="ts">
  // cspell:ignore focalization anachronies

  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import Block from './Block.svelte';
  import Empty from './Empty.svelte';
  import Entry from './Entry.svelte';
  import EntryList from './EntryList.svelte';
  import EntryTitle from './EntryTitle.svelte';
  import EnumChip from './EnumChip.svelte';
  import Field from './Field.svelte';
  import GroupHeading from './GroupHeading.svelte';
  import IdChip from './IdChip.svelte';
  import Quote from './Quote.svelte';
  import type { Narration } from '$lib/feedback/artifacts.ts';

  type Props = { value: Narration };
  const { value }: Props = $props();

  const rows = flex({ direction: 'column', gap: '6px' });
  // 인물 이름·별칭(reflectors·aliases)은 enum이 아니라 임의 텍스트다 — 필(값 토큰) 없이 평문으로, 가운뎃점으로 잇는다.
  const dash = css({ color: 'text.faint' });
</script>

<!-- 상위 키(voice·overtness·focalization…)가 그룹 제목, 그 안의 짧은 키가 행 — 점 경로 대신 파일의 층위를 그대로 편다. -->
<div class={flex({ direction: 'column', gap: '32px' })}>
  <Block>
    <GroupHeading field="narration.voice" label="voice" />
    <div class={rows}>
      <Field field="narration.voice.type" label="type"><EnumChip field="narration.voice.type" value={value.voice.type} /></Field>
      <Field field="narration.voice.note" label="note">{value.voice.note}</Field>
      <Field field="narration.voice.evidence" label="evidence">
        {#if value.voice.evidence.length === 0}
          <span class={dash}>—</span>
        {:else}
          <div class={flex({ direction: 'column', gap: '6px' })}>
            {#each value.voice.evidence as evidence, index (index)}
              <Quote head={evidence} />
            {/each}
          </div>
        {/if}
      </Field>
    </div>
  </Block>

  <Block>
    <GroupHeading field="narration.situation" label="situation" />
    <!-- 스칼라 그룹 — 키 없이 값만. 파일에 없는 키를 지어내지 않는다. -->
    <div><EnumChip field="narration.situation" value={value.situation} /></div>
  </Block>

  <Block>
    <GroupHeading field="narration.overtness" label="overtness" />
    <div class={rows}>
      <Field field="narration.overtness.type" label="type">
        <EnumChip field="narration.overtness.type" value={value.overtness.type} />
      </Field>
      <Field field="narration.overtness.note" label="note">{value.overtness.note}</Field>
    </div>
  </Block>

  <Block>
    <GroupHeading field="narration.focalization" label="focalization" />
    <div class={rows}>
      <Field field="narration.focalization.type" label="type">
        <EnumChip field="narration.focalization.type" value={value.focalization.type} />
      </Field>
      <Field field="narration.focalization.pattern" label="pattern">
        <EnumChip field="narration.focalization.pattern" value={value.focalization.pattern} />
      </Field>
      <Field field="narration.focalization.reflectors" label="reflectors">
        {#if value.focalization.reflectors.length === 0}
          <span class={dash}>—</span>
        {:else}
          {value.focalization.reflectors.join(' · ')}
        {/if}
      </Field>
    </div>
  </Block>

  <Block>
    <GroupHeading field="narration.tense" label="tense" />
    <div class={rows}>
      <Field field="narration.tense.base" label="base">{value.tense.base}</Field>
    </div>
    <GroupHeading field="narration.anachronies" label="anachronies" level={2} />
    {#if value.tense.anachronies.length === 0}
      <Empty text="항목이 없어요" />
    {:else}
      <EntryList>
        {#each value.tense.anachronies as item, index (index)}
          <Entry>
            {#snippet kicker()}
              <IdChip quiet value={item.id} />
            {/snippet}
            <Quote head={item.head} tail={item.tail} />
            <Field field="narration.anachronies.kind" label="kind"><EnumChip field="narration.anachronies.kind" value={item.kind} /></Field>
            <Field field="narration.anachronies.subjectivity" label="subjectivity">
              <EnumChip field="narration.anachronies.subjectivity" value={item.subjectivity} />
            </Field>
            <Field field="narration.anachronies.note" label="note">{item.note}</Field>
          </Entry>
        {/each}
      </EntryList>
    {/if}
  </Block>

  <Block>
    <GroupHeading field="narration.discourse" label="discourse" />
    {#if value.discourse.length === 0}
      <Empty text="항목이 없어요" />
    {:else}
      <EntryList>
        {#each value.discourse as item, index (index)}
          <Entry>
            {#snippet kicker()}
              <IdChip quiet value={item.id} />
            {/snippet}
            <Quote head={item.head} tail={item.tail} />
            <Field field="narration.discourse.form" label="form"><EnumChip field="narration.discourse.form" value={item.form} /></Field>
            <Field field="narration.discourse.note" label="note">{item.note}</Field>
          </Entry>
        {/each}
      </EntryList>
    {/if}
  </Block>

  <Block>
    <GroupHeading field="narration.denomination" label="denomination" />
    {#if value.denomination.length === 0}
      <Empty text="항목이 없어요" />
    {:else}
      <EntryList>
        {#each value.denomination as item, index (index)}
          <Entry>
            {#snippet kicker()}
              <IdChip quiet value={item.id} />
            {/snippet}
            <EntryTitle text={item.name} />
            <Field field="narration.denomination.aliases" label="aliases">
              {#if item.aliases.length === 0}
                <span class={dash}>—</span>
              {:else}
                {item.aliases.join(' · ')}
              {/if}
            </Field>
            {#if item.note}
              <Field field="narration.denomination.note" label="note">{item.note}</Field>
            {/if}
          </Entry>
        {/each}
      </EntryList>
    {/if}
  </Block>

  <Block>
    <GroupHeading field="narration.reliability" label="reliability" />
    <div class={rows}>
      <Field field="narration.reliability.note" label="note">{value.reliability.note}</Field>
    </div>
  </Block>
</div>
