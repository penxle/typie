<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, TextInput } from '@typie/ui/components';
  import { page } from '$app/state';
</script>

<div class={center({ position: 'fixed', inset: '0', paddingX: '20px' })}>
  <form class={flex({ flexDirection: 'column', gap: '16px', width: 'full', maxWidth: '320px' })} method="POST">
    <h1 class={css({ fontSize: '18px', fontWeight: 'extrabold' })}>Bootstrap Bypass</h1>

    <div class={flex({ flexDirection: 'column', gap: '4px' })}>
      <label class={css({ fontSize: '13px', color: 'text.subtle', userSelect: 'none' })} for="key">Bypass Key</label>
      <TextInput id="key" name="key" autocomplete="off" data-1p-ignore placeholder="Enter bypass key" type="password" />
      {#if page.form?.error}
        <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{page.form.error}</div>
      {/if}
    </div>

    <Button style={css.raw({ height: '40px' })} size="lg" type="submit">Enter</Button>
  </form>
</div>
