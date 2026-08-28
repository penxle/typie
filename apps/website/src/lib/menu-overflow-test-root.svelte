<script lang="ts">
  import { contextMenu } from '@typie/ui/actions';
  import { Menu, MenuItem, Select, Submenu } from '@typie/ui/components';

  type Props = {
    itemCount?: number;
    anchorTop?: number;
    anchorLeft?: number;
  };

  let { itemCount = 20, anchorTop = 200, anchorLeft = 40 }: Props = $props();

  const items = $derived(Array.from({ length: itemCount }, (_, index) => ({ label: `Item ${index + 1}`, value: index + 1 })));

  let selected = $state(1);
</script>

{#snippet footer()}
  <div style="padding: 8px 12px">
    <button data-testid="menu-footer-button" type="button">Copy ID</button>
  </div>
{/snippet}

{#snippet contextMenuContent()}
  <Submenu label="Submenu">
    {#each items as item (item.value)}
      <MenuItem>Sub {item.label}</MenuItem>
    {/each}
  </Submenu>
  {#each items as item (item.value)}
    <MenuItem>{item.label}</MenuItem>
  {/each}
  {@render footer()}
{/snippet}

<div data-testid="menu-overflow-test-root">
  <div style:top={`${anchorTop}px`} style:left={`${anchorLeft}px`} style="position: fixed; display: flex; gap: 16px">
    <Menu placement="bottom-start">
      {#snippet button()}
        <span data-testid="menu-trigger">Menu</span>
      {/snippet}

      {#each items as item (item.value)}
        <MenuItem>{item.label}</MenuItem>
      {/each}
      {@render footer()}
    </Menu>

    <div data-testid="select-trigger">
      <Select {items} bind:value={selected} />
    </div>

    <div style="width: 120px; height: 24px" data-testid="context-target" use:contextMenu={{ content: contextMenuContent }}></div>
  </div>
</div>
