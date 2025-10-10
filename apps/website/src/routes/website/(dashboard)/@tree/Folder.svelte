<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { contextMenu } from '@typie/ui/actions';
  import { Icon, Menu } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { tick } from 'svelte';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import { fragment, graphql } from '$graphql';
  import FolderMenu from '../@context-menu/FolderMenu.svelte';
  import EntitySelectionIndicator from './@selection/EntitySelectionIndicator.svelte';
  import MultiEntitiesMenu from './@selection/MultiEntitiesMenu.svelte';
  import Entity from './Entity.svelte';
  import { getTreeContext } from './state.svelte';
  import type { DashboardLayout_EntityTree_Folder_entity, DashboardLayout_EntityTree_Folder_folder, List } from '$graphql';

  type Props = {
    $folder: DashboardLayout_EntityTree_Folder_folder;
    $entities: List<DashboardLayout_EntityTree_Folder_entity>;
  };

  let { $folder: _folder, $entities: _entities }: Props = $props();

  const folder = fragment(
    _folder,
    graphql(`
      fragment DashboardLayout_EntityTree_Folder_folder on Folder {
        id
        name

        entity {
          id
          order
          depth
          visibility
          url

          site {
            id
          }
        }
      }
    `),
  );

  const entities = fragment(
    _entities,
    graphql(`
      fragment DashboardLayout_EntityTree_Folder_entity on Entity {
        id

        ...DashboardLayout_EntityTree_Entity_entity
      }
    `),
  );

  const renameFolder = graphql(`
    mutation DashboardLayout_EntityTree_Folder_RenameFolder_Mutation($input: RenameFolderInput!) {
      renameFolder(input: $input) {
        id
        name
      }
    }
  `);

  const app = getAppContext();
  const treeState = getTreeContext();
  const active = $derived(app.state.ancestors.includes($folder.entity.id));
  const selected = $derived(treeState.selectedEntityIds.has($folder.entity.id));

  let detailsEl = $state<HTMLDetailsElement>();
  let inputEl = $state<HTMLInputElement>();

  let open = $state(false);
  let editing = $state(false);

  $effect(() => {
    if (editing) {
      tick().then(() => {
        inputEl?.select();
      });
    }
  });

  $effect.pre(() => {
    if (active) {
      open = true;
    }
  });

  $effect(() => {
    if (app.state.newFolderId === $folder.id) {
      editing = true;
      app.state.newFolderId = undefined;

      if (detailsEl) {
        const rect = detailsEl.getBoundingClientRect();
        const isInViewport = rect.top >= 0 && rect.bottom <= window.innerHeight;

        if (!isInViewport) {
          detailsEl.scrollIntoView({ behavior: 'instant', block: 'nearest' });
        }
      }
    }
  });
</script>

<details
  bind:this={detailsEl}
  data-id={$folder.entity.id}
  data-order={$folder.entity.order}
  data-path-depth={$folder.entity.depth}
  data-type="folder"
  bind:open
>
  <summary
    class={cx(
      'group',
      css(
        {
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          paddingX: '8px',
          paddingY: '6px',
          borderRadius: '6px',
          transition: 'common',
          cursor: 'pointer',
          _supportHover: { backgroundColor: 'surface.muted' },
          '&:has([aria-pressed="true"])': { backgroundColor: 'surface.muted' },
          '&[data-context-menu-open="true"]': { backgroundColor: 'surface.muted' },
        },
        $folder.entity.depth > 0 && {
          borderLeftWidth: '1px',
          borderLeftRadius: '0',
          marginLeft: '-1px',
          paddingLeft: '14px',
          _supportHover: { borderColor: 'border.strong' },
        },
        selected && {
          backgroundColor: 'accent.brand.subtle',
          _supportHover: { backgroundColor: 'accent.brand.subtle' },
          '&:has([aria-pressed="true"])': { backgroundColor: 'accent.brand.subtle' },
          '&[data-context-menu-open="true"]': { backgroundColor: 'accent.brand.subtle' },
        },
      ),
    )}
    aria-selected="false"
    data-anchor={$entities.length > 0}
    onkeyup={(e) => {
      if (e.code === 'Space') {
        e.preventDefault();
      }
    }}
    role="treeitem"
    use:contextMenu={{ content: contextMenuContent }}
  >
    <EntitySelectionIndicator entityId={$folder.entity.id}>
      {#snippet icon()}
        <Icon style={css.raw({ color: 'text.faint' })} icon={open ? ChevronDownIcon : ChevronRightIcon} size={14} />
      {/snippet}
    </EntitySelectionIndicator>

    {#if editing}
      <form
        class={css({ display: 'contents' })}
        onsubmit={async (e) => {
          e.preventDefault();

          const formData = new FormData(e.currentTarget);

          await renameFolder({
            folderId: $folder.id,
            name: formData.get('name') as string,
          });

          mixpanel.track('rename_folder');

          editing = false;
        }}
      >
        <input
          bind:this={inputEl}
          name="name"
          class={css({
            flexGrow: '1',
            fontSize: '14px',
            fontWeight: 'medium',
            color: 'text.muted',
            minWidth: '0',
          })}
          defaultValue={$folder.name}
          onblur={(e) => e.currentTarget.form?.requestSubmit()}
          onkeydown={(e) => {
            if (e.key === 'Escape') {
              e.preventDefault();
              e.currentTarget.form?.reset();
              editing = false;
            }
          }}
        />
      </form>
    {:else}
      <span
        class={css({
          flexGrow: '1',
          fontSize: '14px',
          fontWeight: 'medium',
          color: 'text.muted',
          wordBreak: 'break-all',
          lineClamp: '1',
        })}
      >
        {$folder.name}
      </span>

      <Menu placement="bottom-start">
        {#snippet button({ open })}
          <div
            class={center({
              borderRadius: '4px',
              size: '16px',
              color: 'text.disabled',
              opacity: '0',
              transition: 'common',
              _hover: { backgroundColor: 'interactive.hover' },
              _groupHover: { opacity: '100' },
              _pressed: { backgroundColor: 'interactive.hover', opacity: '100' },
            })}
            aria-pressed={open}
          >
            <Icon icon={EllipsisIcon} size={14} />
          </div>
        {/snippet}

        {@render contextMenuContent()}
      </Menu>
    {/if}
  </summary>
  {#snippet contextMenuContent()}
    {#if treeState.selectedEntityIds.size > 1 && treeState.selectedEntityIds.has($folder.entity.id)}
      <MultiEntitiesMenu />
    {:else}
      <FolderMenu
        entity={$folder.entity}
        folder={$folder}
        onRename={() => {
          editing = true;
        }}
        open={() => {
          open = true;
        }}
        via="tree"
      />
    {/if}
  {/snippet}

  <div class={flex({ flexDirection: 'column', borderLeftWidth: '1px', marginLeft: '24px' })} aria-hidden={!open} role="tree">
    {#each $entities as entity (entity.id)}
      <Entity $entity={entity} />
    {:else}
      <div class={css({ paddingX: '8px', paddingY: '6px', fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>
        폴더가 비어있어요
      </div>
    {/each}
  </div>
</details>
