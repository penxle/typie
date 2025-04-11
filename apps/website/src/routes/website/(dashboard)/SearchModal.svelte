<script lang="ts">
  import { Dialog } from '$lib/components';

  type Props = {
    open: boolean;
  };

  let { open = $bindable() }: Props = $props();
</script>

<Dialog bind:open>asdf</Dialog>
