import { graphql } from '$mearie';
import type { DataOf } from '@mearie/svelte';

export const undoStateQuery = graphql(`
  query DashboardLayout_PrismSaveDocumentUndo_Query($sessionId: ID!, $toolCallId: String!) {
    prismDocumentEdit(sessionId: $sessionId, toolCallId: $toolCallId) {
      undoable
      undone
      changedAfter
    }
  }
`);

export type UndoState = DataOf<typeof undoStateQuery>['prismDocumentEdit'];
