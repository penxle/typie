<script lang="ts">
  import { findChildren, getText } from '@tiptap/core';
  import { Plugin, PluginKey } from '@tiptap/pm/state';
  import { untrack } from 'svelte';
  import { ySyncPluginKey } from 'y-prosemirror';
  import { textSerializers } from '@/pm/serializer';
  import { fragment, graphql } from '$graphql';
  import PlanUpgradeModal from '../PlanUpgradeModal.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Node } from '@tiptap/pm/model';
  import type { Editor_Limit_query, Editor_Limit_site } from '$graphql';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
    $site: Editor_Limit_site;
    $query: Editor_Limit_query;
  };

  let { $query: _query, $site: _site, editor }: Props = $props();

  const query = fragment(
    _query,
    graphql(`
      fragment Editor_Limit_query on Query {
        defaultPlanRule {
          maxTotalCharacterCount
          maxTotalBlobSize
        }
      }
    `),
  );

  const site = fragment(
    _site,
    graphql(`
      fragment Editor_Limit_site on Site {
        id

        usage {
          totalCharacterCount
          totalBlobSize
        }

        user {
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
      }
    `),
  );

  const siteUsageUpdateStream = graphql(`
    subscription Editor_Limit_SiteUsageUpdateStream($siteId: ID!) {
      siteUsageUpdateStream(siteId: $siteId) {
        ... on Site {
          id

          usage {
            totalCharacterCount
            totalBlobSize
          }
        }
      }
    }
  `);

  const planRule = $derived($site.user.subscription?.plan?.rule ?? $query.defaultPlanRule);

  const totalCharacterCountProgress = $derived.by(() => {
    if (planRule.maxTotalCharacterCount === -1) {
      return -1;
    }

    return Math.min(1, $site.usage.totalCharacterCount / planRule.maxTotalCharacterCount);
  });

  const totalBlobSizeProgress = $derived.by(() => {
    if (planRule.maxTotalBlobSize === -1) {
      return -1;
    }

    return Math.min(1, $site.usage.totalBlobSize / planRule.maxTotalBlobSize);
  });

  let open = $state(false);

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
                open = true;

                return false;
              }
            }

            if (totalBlobSizeProgress >= 1) {
              const oldBlobSize = getBlobSize(state.doc);
              const newBlobSize = getBlobSize(tr.doc);

              if (newBlobSize > oldBlobSize) {
                open = true;

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

  $effect(() => {
    return untrack(() => {
      const unsubscribe = siteUsageUpdateStream.subscribe({ siteId: $site.id });

      return () => {
        unsubscribe();
      };
    });
  });
</script>

<PlanUpgradeModal bind:open>
  현재 플랜의 최대 사용량을 초과했어요.
  <br />
  이어서 작성하려면 플랜을 업그레이드 해주세요.
</PlanUpgradeModal>
