<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { toast as sonner } from 'svelte-sonner';
  import LightbulbIcon from '~icons/lucide/lightbulb';
  import XIcon from '~icons/lucide/x';
  import { Icon } from '../../components';
  import Component from '../sonner/Component.svelte';
  import Shortcut from './Shortcut.svelte';

  type Props = {
    id: string;
    message: string;
    description?: string;
  };

  let { id, message, description, ...rest }: Props = $props();

  const dismiss = () => sonner.dismiss(id);

  const tokens = $derived(message.split(/(`[^`]+`)/g).filter(Boolean));
</script>

<Component {...rest}>
  <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={LightbulbIcon} size={12} />
      <div class={css({ fontSize: '13px', color: 'text.faint' })}>이용 팁</div>
    </div>

    <button onclick={dismiss} type="button">
      <Icon style={css.raw({ color: 'text.faint' })} icon={XIcon} size={12} />
    </button>
  </div>

  <div class={css({ marginTop: '8px', fontSize: '14px', fontWeight: 'medium', color: 'text.subtle', verticalAlign: 'middle' })}>
    {#each tokens as token, index (index)}
      {#if token.startsWith('`') && token.endsWith('`')}
        <Shortcut shortcut={token.slice(1, -1)} />
      {:else}
        <span>{token}</span>
      {/if}
    {/each}
  </div>

  {#if description}
    <div class={css({ fontSize: '13px', color: 'text.faint' })}>{description}</div>
  {/if}
</Component>
