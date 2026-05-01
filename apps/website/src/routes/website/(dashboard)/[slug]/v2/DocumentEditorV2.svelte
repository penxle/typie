<script lang="ts">
  import { createFragment, createMutation, createSubscription } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { onDestroy, onMount } from 'svelte';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  import Editor from '$lib/editor-ffi/components/Editor.svelte';
  import { setupEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { initWasm, wasm } from '$lib/wasm-ffi.svelte';
  import { graphql } from '$mearie';
  import { DebugBus } from './@debug/debug-bus.svelte';
  import DebugPanel from './@debug/DebugPanel.svelte';
  import BottomToolbar from './BottomToolbar.svelte';
  import TopToolbar from './TopToolbar.svelte';
  import type { Doc, ObjectContent, Selection } from '@typie/editor-ffi/browser';
  import type { DocumentEditorV2_document$key } from '$mearie';
  import type { ClientCommitInput, DebugSnapshot, DocumentObjectInput } from './@debug/types';

  type Props = {
    document$key: DocumentEditorV2_document$key;
    slug: string;
  };

  let { document$key, slug }: Props = $props();

  const document = createFragment(
    graphql(`
      fragment DocumentEditorV2_document on Document {
        id
        title

        head {
          id
          hash

          rootObject {
            id
            hash
            content
          }

          objects {
            id
            hash
            content
          }
        }

        ...Editor_document
      }
    `),
    () => document$key,
  );

  const ctx = setupEditorContext();

  const cacheObjects = new SvelteMap<string, ObjectContent>();

  let serverHeadHash = $state<string>('');
  let sinceCommitHash = '';
  let chainTip = $state<string>('');

  let docState = $state<{ doc: Doc; selection: Selection } | null>(null);

  let localCommitChain: ClientCommitInput[] = $state([]);
  let pendingNewObjects: DocumentObjectInput[] = $state([]);
  const pendingPushSet = new SvelteSet<string>(); // 자기 발급 + server head 미흡수 commit hash
  let inflight = $state(false);
  let pushError = $state(false);

  const bus = new DebugBus();
  let debugOpen = $state(true);

  const syncStatus = $derived<'idle' | 'pushing' | 'error'>(pushError ? 'error' : inflight ? 'pushing' : 'idle');

  const snapshot = $derived<DebugSnapshot>({
    serverHeadHash,
    chainTip,
    localCommitChain,
    pendingPushSet,
    inflight,
    syncStatus,
  });

  let idleTimer: ReturnType<typeof setTimeout> | null = null;
  let maxWaitTimer: ReturnType<typeof setTimeout> | null = null;
  const IDLE_MS = 500;
  const MAX_WAIT_MS = 3000;

  const [pushDocumentCommits] = createMutation(
    graphql(`
      mutation DocumentEditorV2_Push($input: PushDocumentCommitsInput!) {
        pushDocumentCommits(input: $input) {
          id
          hash
        }
      }
    `),
  );

  function clearTimers() {
    if (idleTimer) {
      clearTimeout(idleTimer);
      idleTimer = null;
    }

    if (maxWaitTimer) {
      clearTimeout(maxWaitTimer);
      maxWaitTimer = null;
    }
  }

  function schedulePush() {
    if (idleTimer) clearTimeout(idleTimer);
    idleTimer = setTimeout(firePush, IDLE_MS);

    if (!maxWaitTimer && localCommitChain.length > 0) {
      maxWaitTimer = setTimeout(firePush, MAX_WAIT_MS);
    }
  }

  async function firePush() {
    clearTimers();
    if (inflight || localCommitChain.length === 0) return;

    const commits = localCommitChain;
    const objects = pendingNewObjects;

    inflight = true;
    localCommitChain = [];
    pendingNewObjects = [];
    pushError = false;

    bus.emit({ kind: 'push.fired', commits: commits.length, objects: objects.length });
    const startedAt = performance.now();

    try {
      await pushDocumentCommits({
        input: {
          documentId: document.data.id,
          commits,
          objects,
        },
      });
    } catch (err) {
      console.error('pushDocumentCommits failed', err);
      for (const c of commits) pendingPushSet.delete(c.commitHash);
      pushError = true;
      inflight = false;
      bus.emit({ kind: 'push.error', message: String(err) });
      return;
    }

    const durationMs = performance.now() - startedAt;
    bus.emit({ kind: 'push.success', durationMs });

    inflight = false;
    if (localCommitChain.length > 0) schedulePush();
  }

  createSubscription(
    graphql(`
      subscription DocumentEditorV2_Updated($documentId: ID!, $sinceCommitHash: String!) {
        documentCommitsUpdated(documentId: $documentId, sinceCommitHash: $sinceCommitHash) {
          commits {
            id
            hash
            rootObject {
              id
              hash
              content
            }
            committedAt
            pushedAt
          }
          objects {
            id
            hash
            content
          }
        }
      }
    `),
    () => ({ documentId: document.data.id, sinceCommitHash }),
    () => ({
      skip: docState === null,
      onData: (data) => applyEvent(data.documentCommitsUpdated),
    }),
  );

  function applyEvent(event: {
    commits: readonly { id: string; hash: string; rootObject: { hash: string; content: unknown }; committedAt: string; pushedAt: string }[];
    objects: readonly { id: string; hash: string; content: unknown }[];
  }) {
    if (event.commits.length === 0) return;

    const newHead = event.commits.at(-1);
    if (!newHead) return;

    const isOwnHead = pendingPushSet.has(newHead.hash);

    bus.emit({
      kind: 'subscription.received',
      commits: event.commits.length,
      objects: event.objects.length,
      ownEcho: isOwnHead,
      newHead: newHead.hash,
    });

    for (const o of event.objects) {
      cacheObjects.set(o.hash, o.content as ObjectContent);
    }

    for (const c of event.commits) {
      pendingPushSet.delete(c.hash);
    }

    serverHeadHash = newHead.hash;

    if (isOwnHead) {
      return;
    }

    chainTip = newHead.hash;
    localCommitChain = [];
    pendingNewObjects = [];
    pendingPushSet.clear();
    inflight = false;

    docState = buildDocState(newHead.rootObject.hash);
  }

  function buildDocState(rootHash: string): { doc: Doc; selection: Selection } {
    const objectEntries = [...cacheObjects.entries()].map(([hash, content]) => ({ hash, content }));
    const doc = wasm.reconstruct_doc_from_objects(rootHash, objectEntries);

    const rootChildren = (doc.nodes['0'] as { children?: string[] } | undefined)?.children ?? [];
    const firstChildId = rootChildren[0] ?? '0';
    const selection: Selection = {
      anchor: { node_id: firstChildId, offset: 0 },
      head: { node_id: firstChildId, offset: 0 },
    };

    return { doc, selection };
  }

  onMount(async () => {
    const head = document.data.head;
    if (!head) return;

    for (const o of head.objects) {
      cacheObjects.set(o.hash, o.content as ObjectContent);
    }

    serverHeadHash = head.hash;
    sinceCommitHash = head.hash;
    chainTip = head.hash;

    await initWasm();
    docState = buildDocState(head.rootObject.hash);
  });

  $effect(() => {
    if (!ctx.editor) return;

    const off = ctx.editor.on('transaction_committed', (_, { commit }) => {
      const parentCommitHash = chainTip;

      const commitHash = wasm.hash_commit_content({
        parent_hash: parentCommitHash,
        object_hash: commit.root_object_hash,
      });

      chainTip = commitHash;

      localCommitChain.push({
        commitHash,
        parentCommitHash,
        rootObjectHash: commit.root_object_hash,
        steps: commit.steps,
        meta: commit.meta,
        committedAt: new Date(commit.committed_at).toISOString(),
      });

      bus.emit({
        kind: 'commit.created',
        hash: commitHash,
        chainSize: localCommitChain.length,
      });

      pendingNewObjects.push(...commit.objects.map((o) => ({ hash: o.hash, content: o.content })));

      pendingPushSet.add(commitHash);
      schedulePush();
    });

    return off;
  });

  onDestroy(() => {
    clearTimers();
  });
</script>

<div
  class={css({
    display: 'flex',
    flexDirection: 'row',
    height: 'full',
    width: 'full',
  })}
>
  <div
    class={css({
      flex: '1',
      display: 'flex',
      flexDirection: 'column',
      minWidth: '0',
    })}
  >
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
        {#if serverHeadHash}
          <span class={css({ fontSize: '11px', color: 'text.faint', fontFamily: 'mono', fontWeight: 'normal' })}>
            {serverHeadHash.slice(0, 8)}
          </span>
        {/if}
        {#if syncStatus !== 'idle'}
          <span class={css({ fontSize: '11px', color: syncStatus === 'error' ? 'text.danger' : 'text.muted', fontWeight: 'normal' })}>
            {syncStatus}
          </span>
        {/if}
      </h1>
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
          _hover: {
            borderColor: 'border.default',
            backgroundColor: 'surface.muted',
          },
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
    </header>

    {#if !document.data.head}
      <p class={css({ padding: '16px' })}>v1 document</p>
    {:else if docState}
      <TopToolbar />
      <BottomToolbar />
      {#key docState}
        <Editor style={css.raw({ flex: '1' })} doc={docState.doc} document$key={document.data} selection={docState.selection} />
      {/key}
    {/if}
  </div>

  <DebugPanel {bus} onClose={() => (debugOpen = false)} open={debugOpen} {slug} {snapshot} />
</div>
