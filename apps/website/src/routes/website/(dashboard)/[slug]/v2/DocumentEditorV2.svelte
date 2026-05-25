<script lang="ts">
  import { createFragment, createMutation, createSubscription } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { onDestroy, untrack } from 'svelte';
  import EditorComponent from '$lib/editor-ffi/components/Editor.svelte';
  import { setupEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { graphql } from '$mearie';
  import { DebugBus } from './@debug/debug-bus.svelte';
  import DebugPanel from './@debug/DebugPanel.svelte';
  import BottomToolbar from './BottomToolbar.svelte';
  import SettingsPanel from './SettingsPanel.svelte';
  import { Outbox } from './sync/outbox';
  import { Pusher } from './sync/pusher.svelte';
  import TopToolbar from './TopToolbar.svelte';
  import type { DocumentEditorV2_document$key } from '$mearie';
  import type { DebugSnapshot } from './@debug/types';

  type Props = {
    document$key: DocumentEditorV2_document$key;
  };

  let { document$key }: Props = $props();

  const document = createFragment(
    graphql(`
      fragment DocumentEditorV2_document on Document {
        id
        title
        state {
          graph
          updatedAt
        }
        assets {
          __typename

          ... on Image {
            id
            url
            originalUrl
            width
            height
            placeholder
          }

          ... on File {
            id
            name
            size
            url
          }

          ... on Embed {
            id
            url
            title
            description
            thumbnailUrl
            html
          }

          ... on DocumentArchivedNode {
            id
            content
          }
        }
        ...Editor_document
        ...BottomToolbar_document
        ...SettingsPanel_document
      }
    `),
    () => document$key,
  );

  const ctx = setupEditorContext();
  const bus = new DebugBus();
  const clientId = crypto.randomUUID();
  let pusher = $state<Pusher | null>(null);

  let debugOpen = $state(true);
  let settingsOpen = $state(false);

  // Server-confirmed heads. Tracks dots the server is known to have ingested
  // (self-pushed bundles + remote bundles received via subscription/poll).
  // Sent as the `heads` arg in server queries so the server's strict
  // missing_for invariant does not trip on optimistic local dots.
  //
  // Stays null until the editor mounts and the $effect seeds it with
  // editor.currentHeads(). Subscription/poll guard on null so server queries
  // never run with a placeholder — initial subscription connect uses the
  // editor's actual heads (= server graph heads at mount), avoiding catch-up
  // re-fetch of the entire graph that was already loaded via state.graph.
  let lastConfirmedHeads = $state<Uint8Array | null>(null);

  const graph = $derived(document.data.state ? Uint8Array.fromBase64(document.data.state.graph) : null);

  const hasHeads = $derived(!!lastConfirmedHeads);

  const snapshot = $derived<DebugSnapshot>({
    pushStatus: pusher?.status ?? 'idle',
    retryAttempt: pusher?.retryAttempt ?? 0,
    lastSentHeadsBytes: lastConfirmedHeads?.length ?? 0,
    hasEditor: ctx.editor !== undefined,
  });

  const [pushDocumentChangesets] = createMutation(
    graphql(`
      mutation DocumentEditorV2_PushChangesets($input: PushDocumentChangesetsInput!) {
        pushDocumentChangesets(input: $input) {
          heads
        }
      }
    `),
  );

  const [pullDocumentChangesets] = createMutation(
    graphql(`
      mutation DocumentEditorV2_PullChangesets($input: PullDocumentChangesetsInput!) {
        pullDocumentChangesets(input: $input) {
          changesets
          heads
        }
      }
    `),
  );

  createSubscription(
    graphql(`
      subscription DocumentEditorV2_ChangesetsUpdated($documentId: ID!, $clientId: String!, $heads: Binary!) {
        documentChangesetsUpdated(documentId: $documentId, clientId: $clientId, heads: $heads) {
          changesets
          heads
        }
      }
    `),
    // heads is read via untrack: onData writes lastConfirmedHeads, which
    // would otherwise re-trigger args evaluation → mearie reconnect → catch-up
    // emit → onData → loop. Mearie still re-evaluates args on connect/reconnect,
    // so the value seen at that moment is current — we just don't want every
    // confirmed-heads write to cause a forced reconnect.
    //
    // heads asserts non-null because skip guards lastConfirmedHeads === null.
    () => ({
      documentId: document.data.id,
      clientId,
      heads: untrack(() => lastConfirmedHeads?.toBase64() ?? ''),
    }),
    () => ({
      // Defer subscription start until the editor has mounted AND the initial
      // confirmed heads have been seeded — otherwise the first connect would
      // ship empty heads and trigger a full-graph catch-up that duplicates the
      // state.graph already loaded by the page query.
      skip: ctx.editor === undefined || !hasHeads,
      onData: (data) => {
        const editor = ctx.editor;
        if (!editor) return;
        const payload = Uint8Array.fromBase64(data.documentChangesetsUpdated.changesets);
        if (payload.length > 0) {
          editor.receiveRemoteChangeset(payload);
          bus.emit({ kind: 'subscription.received', bytes: payload.length });
        }
        lastConfirmedHeads = Uint8Array.fromBase64(data.documentChangesetsUpdated.heads);
      },
    }),
  );

  $effect(() => {
    const editor = ctx.editor;
    if (!editor) return;

    // Initial confirmed heads = the heads of the server graph the editor was
    // constructed from. Subsequent pushes/receives advance this monotonically.
    // Use a local var instead of reading lastConfirmedHeads back inside the
    // effect — read-after-write inside the same effect re-invalidates it.
    const initialHeads = editor.currentHeads();
    lastConfirmedHeads = initialHeads;

    const outbox = new Outbox(document.data.id);

    const pushFn = async (changesets: Uint8Array) => {
      const result = await pushDocumentChangesets({
        input: {
          documentId: document.data.id,
          clientId,
          changesets: changesets.toBase64(),
        },
      });
      lastConfirmedHeads = Uint8Array.fromBase64(result.pushDocumentChangesets.heads);
    };

    // 재진입 시 outbox에 남은 bundle 재전송
    void (async () => {
      for (const { id, bundle } of await outbox.loadAll()) {
        await pushFn(bundle);
        await outbox.delete(id);
      }
    })();

    const ps = new Pusher({
      editor,
      documentId: document.data.id,
      clientId,
      initialServerHeads: initialHeads,
      pushFn,
      outbox,
      onEvent: (e) => bus.emit(e),
    });
    pusher = ps;

    const offStateChanged = editor.on('state_changed', (_, { fields }) => {
      if (fields.includes('doc')) ps.schedule();
    });

    const pollIntervalId = setInterval(async () => {
      const ed = ctx.editor;
      const heads = lastConfirmedHeads;
      if (!ed || heads === null) return;
      const result = await pullDocumentChangesets({
        input: { documentId: document.data.id, heads: heads.toBase64() },
      });
      const missing = Uint8Array.fromBase64(result.pullDocumentChangesets.changesets);
      if (missing.length > 0) {
        ed.receiveRemoteChangeset(missing);
        bus.emit({ kind: 'poll.applied', bytes: missing.length });
      }
      lastConfirmedHeads = Uint8Array.fromBase64(result.pullDocumentChangesets.heads);
    }, 10_000);

    return () => {
      clearInterval(pollIntervalId);
      offStateChanged();
      ps.stop();
      outbox.destroy();
      pusher = null;
    };
  });

  $effect(() => {
    const editor = ctx.editor;
    if (!editor) return;

    for (const asset of document.data.assets) {
      if (asset.__typename === 'Image') {
        editor.imageAssets.set(asset.id, {
          id: asset.id,
          url: asset.url,
          originalUrl: asset.originalUrl,
          width: asset.width,
          height: asset.height,
          placeholder: asset.placeholder,
        });
      }

      if (asset.__typename === 'File') {
        ctx.fileAssets.set(asset.id, {
          id: asset.id,
          name: asset.name,
          size: asset.size,
          url: asset.url,
        });
      }

      if (asset.__typename === 'Embed') {
        editor.embedAssets.set(asset.id, {
          id: asset.id,
          url: asset.url,
          title: asset.title ?? null,
          description: asset.description ?? null,
          thumbnailUrl: asset.thumbnailUrl ?? null,
          html: asset.html ?? null,
        });
      }

      if (asset.__typename === 'DocumentArchivedNode') {
        editor.archivedAssets.set(asset.id, {
          id: asset.id,
          content: asset.content,
        });
      }
    }
  });

  onDestroy(() => {
    pusher?.stop();
  });
</script>

<div class={css({ display: 'flex', flexDirection: 'row', height: 'full', width: 'full' })}>
  <div class={css({ flex: '1', display: 'flex', flexDirection: 'column', minWidth: '0' })}>
    <header
      class={css({
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        paddingX: '16px',
        paddingY: '12px',
        borderBottomWidth: '1px',
        borderBottomColor: 'border.subtle',
      })}
    >
      <h1 class={css({ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '14px', fontWeight: 'semibold' })}>
        {document.data.title ?? '제목 없음'}
        <span class={css({ color: 'text.muted', fontWeight: 'normal' })}>(v2 dev)</span>
        {#if snapshot.pushStatus !== 'idle'}
          <span
            class={css({
              fontSize: '11px',
              color: snapshot.pushStatus === 'error' ? 'text.danger' : 'text.muted',
              fontWeight: 'normal',
            })}
          >
            {snapshot.pushStatus}
          </span>
        {/if}
      </h1>
      <div class={css({ display: 'flex', alignItems: 'center', gap: '8px' })}>
        <button
          class={css({
            display: 'inline-flex',
            alignItems: 'center',
            gap: '6px',
            paddingX: '10px',
            paddingY: '4px',
            borderRadius: '[10px]',
            borderWidth: '1px',
            borderColor: settingsOpen ? 'border.default' : 'border.subtle',
            backgroundColor: settingsOpen ? 'surface.muted' : 'transparent',
            color: settingsOpen ? 'text.default' : 'text.muted',
            cursor: 'pointer',
            fontFamily: 'ui',
            fontSize: '10px',
            fontWeight: 'semibold',
            letterSpacing: '[0.12em]',
            textTransform: 'uppercase',
            transition: '[all 120ms]',
            _hover: { borderColor: 'border.default', backgroundColor: 'surface.muted' },
          })}
          aria-pressed={settingsOpen}
          onclick={() => (settingsOpen = !settingsOpen)}
          type="button"
        >
          <span
            class={css({
              width: '6px',
              height: '6px',
              borderRadius: 'full',
              backgroundColor: settingsOpen ? 'text.default' : 'border.default',
              transition: '[background-color 120ms]',
            })}
          ></span>
          Settings
        </button>
        <button
          class={css({
            display: 'inline-flex',
            alignItems: 'center',
            gap: '6px',
            paddingX: '10px',
            paddingY: '4px',
            borderRadius: '[10px]',
            borderWidth: '1px',
            borderColor: debugOpen ? 'border.default' : 'border.subtle',
            backgroundColor: debugOpen ? 'surface.muted' : 'transparent',
            color: debugOpen ? 'text.default' : 'text.muted',
            cursor: 'pointer',
            fontFamily: 'ui',
            fontSize: '10px',
            fontWeight: 'semibold',
            letterSpacing: '[0.12em]',
            textTransform: 'uppercase',
            transition: '[all 120ms]',
            _hover: { borderColor: 'border.default', backgroundColor: 'surface.muted' },
          })}
          aria-pressed={debugOpen}
          onclick={() => (debugOpen = !debugOpen)}
          type="button"
        >
          <span
            class={css({
              width: '6px',
              height: '6px',
              borderRadius: 'full',
              backgroundColor: debugOpen ? 'text.default' : 'border.default',
              transition: '[background-color 120ms]',
            })}
          ></span>
          Debug
        </button>
      </div>
    </header>

    <TopToolbar />
    <BottomToolbar document$key={document.data} />
    {#if graph}
      <EditorComponent style={css.raw({ flex: '1' })} document$key={document.data} {graph} />
    {/if}
  </div>

  <SettingsPanel document$key={document.data} onClose={() => (settingsOpen = false)} open={settingsOpen} />
  <DebugPanel {bus} onClose={() => (debugOpen = false)} open={debugOpen} {snapshot} />
</div>
