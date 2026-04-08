<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { Icon } from '@typie/ui/components';
  import FileIcon from '~icons/lucide/file';
  import { graphql } from '$mearie';
  import { entityIconMap, getEntityIconColor } from './entity-icons';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Component, ComponentProps } from 'svelte';
  import type { EntityIcon_entity$key } from '$mearie';

  type Props = {
    entity$key: EntityIcon_entity$key;
    fallback?: Component;
    size?: ComponentProps<typeof Icon>['size'];
    style?: SystemStyleObject;
  };

  let { entity$key, fallback = FileIcon, size = 14, style }: Props = $props();

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
</script>

<span style:color={getEntityIconColor(entity.data.iconColor)}>
  <Icon {style} icon={entityIconMap.get(entity.data.icon) ?? fallback} {size} />
</span>
