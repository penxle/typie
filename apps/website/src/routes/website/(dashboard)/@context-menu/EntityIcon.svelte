<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { Icon } from '@typie/ui/components';
  import { entityIconMap, getEntityIconColor } from '@typie/ui/constants';
  import FileIcon from '~icons/lucide/file';
  import { graphql } from '$mearie';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Component, ComponentProps } from 'svelte';
  import type { EntityIcon_entity$key } from '$mearie';

  type SharedProps = {
    fallback?: Component;
    size?: ComponentProps<typeof Icon>['size'];
    style?: SystemStyleObject;
  };

  type Props = SharedProps &
    ({ entity$key: EntityIcon_entity$key; icon?: never; iconColor?: never } | { entity$key?: never; icon: string; iconColor: string });

  let { entity$key, icon, iconColor, fallback = FileIcon, size = 14, style }: Props = $props();

  const entity = createFragment(
    graphql(`
      fragment EntityIcon_entity on Entity {
        id
        icon
        iconColor
      }
    `),
    () => entity$key,
  );

  const resolvedIcon = $derived(entity.data?.icon ?? icon ?? '');
  const resolvedIconColor = $derived(entity.data?.iconColor ?? iconColor ?? '');
</script>

<span style:color={getEntityIconColor(resolvedIconColor)}>
  <Icon {style} icon={entityIconMap.get(resolvedIcon) ?? fallback} {size} />
</span>
