<script lang="ts">
  import { setClient } from '@mearie/svelte';
  import { NotificationProvider } from '@typie/ui/notification';
  import { mearieClient } from '$lib/graphql';
  import DocumentSaveDialog from './DocumentSaveDialog.svelte';
  import DocumentSaveIndicator from './DocumentSaveIndicator.svelte';
  import { documentEditing } from './state.svelte';
  import type { DocumentEditingSession } from './session';

  let { session }: { session?: DocumentEditingSession } = $props();

  setClient(mearieClient);
</script>

<NotificationProvider />
<DocumentSaveDialog />
{#if session}
  <DocumentSaveIndicator
    inspectedStatus={session.inspectedSaveStatus}
    onShowDetails={() => session && documentEditing.showSaveStatus([session])}
    protectedChanges={session.protectedChanges}
    status={session.saveStatus}
    unconfirmedSince={session.unconfirmedSince}
    unprotectedSince={session.unprotectedSince}
  />
{/if}
