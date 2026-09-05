<script lang="ts">
  import { EntityAvailability, EntityVisibility } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import DotIcon from '~icons/lucide/dot';

  type Props = {
    availability: EntityAvailability;
    characterCount?: number;
    createdAt: string;
    updatedAt: string;
    visibility: EntityVisibility;
  };

  let { availability, characterCount, createdAt, updatedAt, visibility }: Props = $props();
</script>

<div class={flex({ flexDirection: 'column', gap: '4px', minWidth: '140px' })}>
  <div class={flex({ alignItems: 'center', gap: '4px', fontWeight: 'semibold' })}>
    {#if visibility === EntityVisibility.PUBLIC}
      <span>공개 조회</span>
    {:else if visibility === EntityVisibility.UNLISTED}
      <span>링크 조회</span>
    {:else}
      <span>비공개</span>
    {/if}

    <Icon icon={DotIcon} size={12} />

    {#if availability === EntityAvailability.UNLISTED}
      <span>링크 편집</span>
    {:else}
      <span>나만 편집</span>
    {/if}
  </div>

  {#if characterCount !== undefined}
    <div>총 {comma(characterCount)}자</div>
  {/if}

  <div class={css({ opacity: '50' })}>
    <div>생성: {dayjs(createdAt).formatAsDateTime()}</div>
    <div>수정: {dayjs(updatedAt).formatAsDateTime()}</div>
  </div>
</div>
