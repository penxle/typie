<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon } from '@typie/ui/components';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import type { Snippet } from 'svelte';

  type Props = {
    title: string;
    subtitle?: string | null;
    hasPassword: boolean;
    paginated: boolean;
    titleEl?: HTMLElement;
    actions: Snippet;
  };

  let { title, subtitle, hasPassword, paginated, titleEl = $bindable(), actions }: Props = $props();
</script>

<div class={flex({ alignItems: 'flex-start', justifyContent: 'space-between', gap: '16px' })}>
  <div class={flex({ flexDirection: 'column', minWidth: '0' })}>
    <h1
      bind:this={titleEl}
      class={css({
        fontSize: { base: '26px', md: '32px' },
        fontWeight: 'bold',
        lineHeight: '[1.35]',
        letterSpacing: '-0.02em',
        textWrap: 'balance',
      })}
    >
      {title}
    </h1>

    {#if subtitle}
      <p
        class={css({
          marginTop: '8px',
          fontSize: { base: '15px', md: '17px' },
          fontWeight: 'medium',
          lineHeight: '[1.5]',
          color: 'text.muted',
        })}
      >
        {subtitle}
      </p>
    {/if}

    {#if hasPassword}
      <div
        class={flex({
          alignItems: 'center',
          gap: '5px',
          marginTop: '16px',
          width: 'fit',
          height: '26px',
          paddingX: '10px',
          borderRadius: 'full',
          borderWidth: '1px',
          borderColor: 'border.hairline',
          fontSize: '12px',
          color: 'text.muted',
        })}
      >
        <Icon icon={LockOpenIcon} size={12} />
        <span>비밀번호 확인 후 열람 중</span>
      </div>
    {/if}
  </div>

  <div
    class={flex({
      flexShrink: '0',
      alignItems: 'center',
      gap: '2px',
      marginTop: '4px',
      marginRight: '-6px',
      color: 'text.muted',
    })}
  >
    {@render actions()}
  </div>
</div>

{#if paginated}
  <div class={css({ height: '32px' })}></div>
{:else}
  <HorizontalDivider style={css.raw({ marginTop: '32px', marginBottom: '32px' })} color="secondary" />
{/if}
