<script lang="ts">
  import { Icon } from '@typie/ui/components';
  import { emojis } from './emoji';

  type Props = {
    emoji: string;
    size?: 16 | 18 | 20;
  };

  let { emoji, size = 16 }: Props = $props();
</script>

<Icon icon={emojis[emoji]} {size} />
