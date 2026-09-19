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
    onShowDetails={() => session && documentEditing.showSaveStatus([session])}
    protectedChanges={session.isProtected()}
    status={session.saveStatus}
  />
{/if}
