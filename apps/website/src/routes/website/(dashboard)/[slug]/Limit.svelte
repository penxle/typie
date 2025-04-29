<script lang="ts">
  import { findChildren, getText } from '@tiptap/core';
  import { Plugin, PluginKey } from '@tiptap/pm/state';
  import { untrack } from 'svelte';
  import { ySyncPluginKey } from 'y-prosemirror';
  import { textSerializers } from '@/pm/serializer';
  import { getAppContext } from '$lib/context';
  import type { Editor } from '@tiptap/core';
  import type { Node } from '@tiptap/pm/model';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
  };

  let { editor }: Props = $props();

  const app = getAppContext();
  const key = new PluginKey('limit');

  const getCharacterCount = (node: Node) => {
    const text = getText(node, {
      blockSeparator: '\n',
      textSerializers,
    });

    return [...text.replaceAll(/\s+/g, ' ').trim()].length;
  };

  const getBlobSize = (node: Node) => {
    const sizes = findChildren(node, (node) => node.type.name === 'file' || node.type.name === 'image').map(
      ({ node }) => Number(node.attrs.size) || 0,
    );
    return sizes.reduce((acc, size) => acc + size, 0);
  };

  $effect(() => {
    return untrack(() => {
      editor?.current.registerPlugin(
        new Plugin({
          key,
          filterTransaction: (tr, state) => {
            if (!tr.docChanged) {
              return true;
            }

            if (tr.getMeta(ySyncPluginKey)) {
              return true;
            }

            if (app.state.progress.totalCharacterCount >= 1) {
              const oldCharacterCount = getCharacterCount(state.doc);
              const newCharacterCount = getCharacterCount(tr.doc);

              return newCharacterCount <= oldCharacterCount;
            }

            if (app.state.progress.totalBlobSize >= 1) {
              const oldBlobSize = getBlobSize(state.doc);
              const newBlobSize = getBlobSize(tr.doc);

              return newBlobSize <= oldBlobSize;
            }

            return true;
          },
        }),
      );

      return () => {
        editor?.current.unregisterPlugin(key);
      };
    });
  });
</script>
