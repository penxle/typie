<script lang="ts">
  import { Icon } from '$lib/components';
  import { emojis } from './emoji';

  type Props = {
    emoji: string;
  };

  let { emoji }: Props = $props();
</script>

<Icon icon={emojis[emoji]} />
