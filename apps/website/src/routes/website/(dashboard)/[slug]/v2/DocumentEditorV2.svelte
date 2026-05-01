<script lang="ts">
  import { createFragment, createMutation, createSubscription } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { onDestroy, onMount } from 'svelte';
  import Editor from '$lib/editor-ffi/components/Editor.svelte';
  import { setupEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { initWasm, wasm } from '$lib/wasm-ffi.svelte';
  import { graphql } from '$mearie';
  import { DebugBus } from './@debug/debug-bus.svelte';
  import DebugPanel from './@debug/DebugPanel.svelte';
  import BottomToolbar from './BottomToolbar.svelte';
  import { Outbox } from './sync/outbox.svelte';
  import { Pusher } from './sync/pusher.svelte';
  import TopToolbar from './TopToolbar.svelte';
  import type { Doc, ObjectContent, Selection } from '@typie/editor-ffi/browser';
  import type { DocumentEditorV2_document$key } from '$mearie';
  import type { DebugSnapshot } from './@debug/types';
  import type { ClientCommitInput, DocumentObjectInput } from './sync/types';

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

  // 디버그 스냅샷에 노출되지만 반응적으로 읽히지 않으므로 plain Map.
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  const cacheObjects = new Map<string, ObjectContent>();

  let serverHeadHash = $state<string>('');
  let sinceCommitHash = $state<string>('');
  let chainTip = $state<string>('');

  let docState = $state<{ doc: Doc; selection: Selection } | null>(null);

  let outbox = $state<Outbox | null>(null);
  let pusher = $state<Pusher | null>(null);

  const bus = new DebugBus();
  let debugOpen = $state(true);

  const snapshot = $derived<DebugSnapshot>({
    serverHeadHash,
    chainTip,
    outbox: outbox?.entries ?? [],
    cacheObjects,
    pushStatus: pusher?.status ?? 'idle',
    retryAttempt: pusher?.retryAttempt ?? 0,
    hasDocState: docState !== null,
  });

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

  async function pushFn(input: { documentId: string; commits: ClientCommitInput[]; objects: DocumentObjectInput[] }): Promise<void> {
    await pushDocumentCommits({ input });
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
      skip: docState === null || outbox === null,
      onData: (data) => {
        void applyEvent(data.documentCommitsUpdated);
      },
    }),
  );

  async function applyEvent(event: {
    commits: readonly { id: string; hash: string; rootObject: { hash: string; content: unknown }; committedAt: string; pushedAt: string }[];
    objects: readonly { id: string; hash: string; content: unknown }[];
  }) {
    if (event.commits.length === 0) return;
    if (!outbox) return;

    const newHead = event.commits.at(-1);
    if (!newHead) return;

    const isOwnHead = outbox.hasPending(newHead.hash);

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

    await outbox.prune(event.commits.map((c) => c.hash));
    pusher?.notifyEcho(event.commits.map((c) => c.hash));

    serverHeadHash = newHead.hash;

    if (isOwnHead) {
      pusher?.schedule();
      return;
    }

    await outbox.clear();
    pusher?.notifyClear();
    chainTip = newHead.hash;
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

    await initWasm();

    const ob = await Outbox.open(document.data.id);

    for (const e of ob.entries) {
      for (const o of e.objects) {
        cacheObjects.set(o.hash, o.content as ObjectContent);
      }
    }

    const lastEntry = ob.entries.at(-1);
    chainTip = lastEntry?.commit.commitHash ?? head.hash;
    const ownDocRootHash = lastEntry?.commit.rootObjectHash ?? head.rootObject.hash;

    sinceCommitHash = ob.firstParentCommitHash() ?? head.hash;

    const ps = new Pusher({
      documentId: document.data.id,
      outbox: ob,
      push: pushFn,
      onEvent: (e) => bus.emit(e),
    });

    outbox = ob;
    pusher = ps;
    docState = buildDocState(ownDocRootHash);
    ps.flushNow();
  });

  $effect(() => {
    if (!ctx.editor || !pusher) return;
    const ps = pusher;

    const off = ctx.editor.on('transaction_committed', async (_, { commit }) => {
      const parentCommitHash = chainTip;
      const commitHash = wasm.hash_commit_content({
        parent_hash: parentCommitHash,
        object_hash: commit.root_object_hash,
      });

      chainTip = commitHash;

      try {
        await ps.append({
          commit: {
            commitHash,
            parentCommitHash,
            rootObjectHash: commit.root_object_hash,
            steps: commit.steps,
            meta: commit.meta,
            committedAt: new Date(commit.committed_at).toISOString(),
          },
          objects: commit.objects.map((o) => ({ hash: o.hash, content: o.content })),
        });
      } catch {
        // Pusher already set status to 'error'; swallow to avoid unhandled rejection from editor handler.
      }

      bus.emit({ kind: 'commit.created', hash: commitHash, chainSize: outbox?.pendingSize ?? 0 });
    });

    return off;
  });

  onDestroy(() => {
    pusher?.stop();
    outbox?.close();
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
