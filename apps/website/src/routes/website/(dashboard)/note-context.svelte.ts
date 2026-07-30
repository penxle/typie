import { createSubscription } from '@mearie/svelte';
import { browser } from '$app/environment';
import { cache } from '$lib/graphql';
import { NoteEdits, setNoteEditsContext } from '$lib/note/note-edit-state.svelte';
import { NoteOperations, setNoteOperationsContext } from '$lib/note/note-mutation';
import { NoteSync, setNoteSyncContext } from '$lib/note/note-sync.svelte';
import { graphql } from '$mearie';
import { SubscribeModal } from './@subscription/subscribe-modal.svelte';

export function setupNoteContext(getSiteId: () => string): void {
  const clientId = browser ? crypto.randomUUID() : '';
  const noteSync = setNoteSyncContext(
    new NoteSync({
      invalidateGlobal: () => cache.invalidate({ __typename: 'Query', $field: 'notes' }),
      invalidateEntity: (_siteId, entityId) => cache.invalidate({ __typename: 'Entity', id: entityId, $field: 'notes' }),
    }),
  );
  const noteOperations = setNoteOperationsContext(
    new NoteOperations({
      clientId,
      sync: noteSync,
      admit: (via) => SubscribeModal.gate(via),
    }),
  );
  const noteEdits = setNoteEditsContext(
    new NoteEdits({
      isTerminallyDeleted: (siteId, noteId) => noteSync.isTerminallyDeleted(siteId, noteId),
      save: async ({ siteId, noteId, field, value }) => {
        const outcome = await noteOperations.update(field === 'content' ? { noteId, content: value } : { noteId, color: value }, {
          lastKnown: { siteId, noteId },
        });
        if (outcome.status === 'success') {
          return {
            kind: 'saved',
            snapshot: {
              content: outcome.value.content,
              color: outcome.value.color,
            },
          };
        }
        if (outcome.status === 'subscription_gated') {
          return { kind: 'subscription_gated' };
        }
        return outcome.status === 'not_found' ? { kind: 'not_found' } : { kind: 'failed' };
      },
    }),
  );

  $effect(() =>
    noteSync.onTerminalDelete({
      siteId: getSiteId(),
      listener: (noteId) => noteEdits.remove(noteId),
    }),
  );

  createSubscription(
    graphql(`
      subscription NoteContext_NoteUpdateStream_Subscription($siteId: ID!, $clientId: String!) {
        noteUpdateStream(siteId: $siteId, clientId: $clientId) {
          kind
          noteId
        }
      }
    `),
    () => ({ siteId: getSiteId(), clientId }),
    () => {
      const siteId = getSiteId();
      return {
        skip: !browser,
        onData: (data) => noteSync.receiveRemote({ ...data.noteUpdateStream, siteId }),
      };
    },
  );
}
