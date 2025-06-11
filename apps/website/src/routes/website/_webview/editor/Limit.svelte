<script lang="ts">
  import { findChildren, getText } from '@tiptap/core';
  import { Plugin, PluginKey } from '@tiptap/pm/state';
  import { untrack } from 'svelte';
  import { ySyncPluginKey } from 'y-prosemirror';
  import { textSerializers } from '@/pm/serializer';
  import { fragment, graphql } from '$graphql';
  import type { Editor } from '@tiptap/core';
  import type { Node } from '@tiptap/pm/model';
  import type { WebViewEditor_Limit_query } from '$graphql';
  import type { Ref } from '$lib/utils';

  type Props = {
    $query: WebViewEditor_Limit_query;
    editor?: Ref<Editor>;
  };

  let { $query: _query, editor }: Props = $props();

  const query = fragment(
    _query,
    graphql(`
      fragment WebViewEditor_Limit_query on Query {
        defaultPlanRule {
          maxTotalCharacterCount
          maxTotalBlobSize
        }

        me @required {
          id

          subscription {
            id

            plan {
              id

              rule {
                maxTotalCharacterCount
                maxTotalBlobSize
              }
            }
          }
        }

        site(siteId: $siteId) {
          id

          usage {
            totalCharacterCount
            totalBlobSize
          }
        }
      }
    `),
  );

  const planRule = $derived($query.me.subscription?.plan?.rule ?? $query.defaultPlanRule);

  const totalCharacterCountProgress = $derived.by(() => {
    if (planRule.maxTotalCharacterCount === -1) {
      return -1;
    }

    return Math.min(1, $query.site.usage.totalCharacterCount / planRule.maxTotalCharacterCount);
  });

  const totalBlobSizeProgress = $derived.by(() => {
    if (planRule.maxTotalBlobSize === -1) {
      return -1;
    }

    return Math.min(1, $query.site.usage.totalBlobSize / planRule.maxTotalBlobSize);
  });

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

            if (totalCharacterCountProgress >= 1) {
              const oldCharacterCount = getCharacterCount(state.doc);
              const newCharacterCount = getCharacterCount(tr.doc);

              if (newCharacterCount > oldCharacterCount) {
                window.__webview__?.emitEvent('limitExceeded');
                return false;
              }
            }

            if (totalBlobSizeProgress >= 1) {
              const oldBlobSize = getBlobSize(state.doc);
              const newBlobSize = getBlobSize(tr.doc);

              if (newBlobSize > oldBlobSize) {
                window.__webview__?.emitEvent('limitExceeded');
                return false;
              }
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
