<script lang="ts">
  import { cacheExchange, createClient } from '@mearie/core';
  import { filter, map, pipe } from '@mearie/core/stream';
  import { setClient } from '@mearie/svelte';
  import { onDestroy, setContext } from 'svelte';
  import { schema } from '$mearie';
  import EditorBreadcrumbNavigation from './EditorBreadcrumbNavigation.svelte';
  import type { Exchange, OperationResult } from '@mearie/core';
  import type { EditorContextBarSegmentState } from '$lib/editor-ffi/components/ui/editor-context-bar.svelte';
  import type { EntityIcon_entity$key } from '$mearie';
  import type { DragItem, DragPane, DropZone, PaneGroup } from '../../@pane/context.svelte';
  import type { EditorBreadcrumbPathEntity } from './EditorBreadcrumbNavigation.svelte';

  const currentDocument = {
    __typename: 'Entity',
    id: 'document-current',
    slug: 'document-current',
    icon: null,
    iconColor: 'gray',
    node: { __typename: 'Document', id: 'document-node-current', title: 'Current document' },
  };

  const siblingDocument = {
    __typename: 'Entity',
    id: 'document-sibling',
    slug: 'document-sibling',
    icon: null,
    iconColor: 'gray',
    node: { __typename: 'Document', id: 'document-node-sibling', title: 'Sibling document' },
  };

  const rootEntities = [
    {
      __typename: 'Entity',
      id: 'folder-1',
      slug: 'folder-1',
      icon: null,
      iconColor: 'gray',
      node: { __typename: 'Folder', id: 'folder-node-1', name: 'Folder' },
    },
    {
      __typename: 'Entity',
      id: 'document-first',
      slug: 'document-first',
      icon: null,
      iconColor: 'gray',
      node: { __typename: 'Document', id: 'document-node-first', title: 'First document' },
    },
    ...Array.from({ length: 16 }, (_, index) => ({
      __typename: 'Entity',
      id: `document-extra-${index}`,
      slug: `document-extra-${index}`,
      icon: null,
      iconColor: 'gray',
      node: { __typename: 'Document', id: `document-node-extra-${index}`, title: `Extra document ${index}` },
    })),
  ];

  const childEntities = [siblingDocument, currentDocument];
  const fixtureExchange: Exchange = () => ({
    name: 'breadcrumb-document-drag-fixture',
    io: (operations) =>
      pipe(
        operations,
        filter((operation) => operation.variant === 'request'),
        map((operation): OperationResult => {
          return {
            operation,
            data:
              operation.variant === 'request' && operation.artifact.name === 'EditorBreadcrumbNavigation_SiteEntities_Query'
                ? { site: { __typename: 'Site', id: 'site-1', entities: rootEntities } }
                : { entity: { __typename: 'Entity', id: 'folder-1', children: childEntities } },
          };
        }),
      ),
  });

  const client = createClient({
    schema,
    exchanges: [cacheExchange(), fixtureExchange],
    scalars: {
      JSON: { parse: (value: unknown) => value, serialize: (value: unknown) => value },
      Binary: { parse: String, serialize: (value: string) => value },
      DateTime: { parse: String, serialize: (value: string) => value },
      BigInt: { parse: String, serialize: (value: string) => value },
    },
  });

  setClient(client);
  onDestroy(() => client.dispose());

  let nextDropZone = $state<DropZone | 'none'>('center');
  let paneActiveZone = $state<PaneGroup['activeZone']>(null);
  let updates = $state<{ x: number; y: number; zone: DropZone | null }[]>([]);
  let executions = $state<{ item: DragItem | DragPane; zone: DropZone | null }[]>([]);
  let cancelCount = $state(0);
  let holds = $state<string[]>([]);
  let navigatedSlug = $state('');
  const ancestors: EditorBreadcrumbPathEntity[] = [
    {
      id: 'folder-1',
      name: 'Folder',
      entity$key: rootEntities[0] as unknown as EntityIcon_entity$key,
    },
  ];

  const paneGroup = {
    get activeZone() {
      return paneActiveZone;
    },
    set activeZone(value: PaneGroup['activeZone']) {
      paneActiveZone = value;
    },
    updateActiveZone(x: number, y: number) {
      paneActiveZone = nextDropZone === 'none' ? null : { paneId: 'pane-1', dropZone: nextDropZone };
      updates = [...updates, { x, y, zone: paneActiveZone?.dropZone ?? null }];
    },
    executeDrop(item: DragItem | DragPane) {
      executions = [...executions, { item, zone: paneActiveZone?.dropZone ?? null }];
      paneActiveZone = null;
      return true;
    },
    cancelDrag() {
      cancelCount++;
      paneActiveZone = null;
    },
  } as PaneGroup;

  setContext(Symbol.for('typie.svelte-context.pane.PaneGroup'), paneGroup);

  const segment = {
    hold(reason: string) {
      if (!holds.includes(reason)) holds = [...holds, reason];
    },
    release(reason: string) {
      holds = holds.filter((hold) => hold !== reason);
    },
  } as EditorContextBarSegmentState;
</script>

<label>
  zone
  <select data-next-drop-zone bind:value={nextDropZone}>
    <option value="center">center</option>
    <option value="left">left</option>
    <option value="right">right</option>
    <option value="top">top</option>
    <option value="bottom">bottom</option>
    <option value="none">none</option>
  </select>
</label>
<EditorBreadcrumbNavigation
  {ancestors}
  current={{
    id: 'document-current',
    slug: 'document-current',
    name: 'Current document',
    entity$key: currentDocument as unknown as EntityIcon_entity$key,
  }}
  isOwner
  onNavigate={(slug) => (navigatedSlug = slug)}
  popupId="breadcrumb-document-drag-test"
  {segment}
  siteId="site-1"
/>

<output data-pane-updates>{JSON.stringify(updates)}</output>
<output data-pane-executions>{JSON.stringify(executions)}</output>
<output data-pane-cancel-count>{cancelCount}</output>
<output data-holds>{JSON.stringify(holds)}</output>
<output data-navigated-slug>{navigatedSlug}</output>
