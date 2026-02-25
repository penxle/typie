<script lang="ts">
  import { createFragment, createSubscription } from '@mearie/svelte';
  import { findChildren, getText } from '@tiptap/core';
  import { Plugin, PluginKey } from '@tiptap/pm/state';
  import { untrack } from 'svelte';
  import { ySyncPluginKey } from 'y-prosemirror';
  import { textSerializers } from '@/pm/serializer';
  import { graphql } from '$mearie';
  import PlanUpgradeModal from '../PlanUpgradeModal.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Node } from '@tiptap/pm/model';
  import type { Ref } from '@typie/ui/utils';
  import type { Editor_Limit_query$key, Editor_Limit_user$key } from '$mearie';

  type Props = {
    editor?: Ref<Editor>;
    user$key: Editor_Limit_user$key;
    query$key: Editor_Limit_query$key;
  };

  let { query$key, user$key, editor }: Props = $props();

  const query = createFragment(
    graphql(`
      fragment Editor_Limit_query on Query {
        defaultPlanRule {
          maxTotalCharacterCount
          maxTotalBlobSize
        }
      }
    `),
    () => query$key,
  );

  const user = createFragment(
    graphql(`
      fragment Editor_Limit_user on User {
        id

        ...DashboardLayout_PlanUpgradeModal_user

        usage {
          totalCharacterCount
          totalBlobSize
        }

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
    `),
    () => user$key,
  );

  createSubscription(
    graphql(`
      subscription Editor_Limit_UserUsageUpdateStream($userId: ID!) {
        userUsageUpdateStream(userId: $userId) {
          id

          usage {
            totalCharacterCount
            totalBlobSize
          }
        }
      }
    `),
    () => ({ userId: user.data.id }),
  );

  const planRule = $derived(user.data.subscription?.plan?.rule ?? query.data.defaultPlanRule);

  const totalCharacterCountProgress = $derived.by(() => {
    if (planRule.maxTotalCharacterCount === -1) {
      return -1;
    }

    return Math.min(1, user.data.usage.totalCharacterCount / planRule.maxTotalCharacterCount);
  });

  const totalBlobSizeProgress = $derived.by(() => {
    if (planRule.maxTotalBlobSize === -1) {
      return -1;
    }

    return Math.min(1, Number(user.data.usage.totalBlobSize) / planRule.maxTotalBlobSize);
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
</script>

<PlanUpgradeModal user$key={user.data} bind:open>
  현재 플랜의 최대 사용량을 초과했어요.
  <br />
  이어서 작성하려면 플랜을 업그레이드 해주세요.
</PlanUpgradeModal>
