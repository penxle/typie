<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, SearchableDropdown } from '@typie/ui/components';
  import { defaultValues, values } from '@typie/ui/tiptap';
  import mixpanel from 'mixpanel-browser';
  import PlusIcon from '~icons/lucide/plus';
  import { fragment, graphql } from '$graphql';
  import FontUploadModal from '../../FontUploadModal.svelte';
  import PlanUpgradeModal from '../../PlanUpgradeModal.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';
  import type { Editor_BottomToolbar_FontFamily_user } from '$graphql';

  type Props = {
    $user: Editor_BottomToolbar_FontFamily_user;
    editor?: Ref<Editor>;
  };

  let { $user: _user, editor }: Props = $props();

  let uploadModalOpen = $state(false);
  let planUpgradeOpen = $state(false);

  const user = fragment(
    _user,
    graphql(`
      fragment Editor_BottomToolbar_FontFamily_user on User {
        id

        fontFamilies {
          id
          name

          fonts {
            id
            weight
          }
        }

        subscription {
          id
        }
      }
    `),
  );

  const currentFontFamilyValue = $derived.by(() => {
    const value = editor?.current.getAttributes('text_style').fontFamily;

    // NOTE: 레거시 지원; value가 font id(FONT0)인 경우, font family id(FNTF0)로 변환
    for (const fontFamily of $user.fontFamilies) {
      if (fontFamily.fonts.some((font) => font.id === value)) {
        return fontFamily.id;
      }
    }

    return value ?? defaultValues.fontFamily;
  });

  const allFontFamilies = $derived.by(() => {
    const systemFonts = values.fontFamily.map((f) => ({ value: f.value, label: f.label }));
    const userFonts = $user.subscription ? $user.fontFamilies.map((f) => ({ value: f.id, label: f.name })) : [];
    return [...systemFonts, ...userFonts];
  });

  const getDefaultWeight = (fontFamilyOrId: string, fontWeight: number) => {
    let weights: number[];

    const systemFontFamily = values.fontFamily.find((f) => f.value === fontFamilyOrId);
    if (systemFontFamily) {
      weights = systemFontFamily.weights.toSorted((a, b) => a - b);
    } else {
      const userFontFamily = $user.fontFamilies.find((f) => f.id === fontFamilyOrId);
      if (!userFontFamily) return null;

      weights = userFontFamily.fonts.map((f) => f.weight).toSorted((a, b) => a - b);
    }

    if (weights.length === 0) return null;

    if (weights.includes(fontWeight)) {
      return fontWeight;
    }

    let closest = weights[0];
    let minDiff = Math.abs(fontWeight - weights[0]);

    for (const weight of weights) {
      const diff = Math.abs(fontWeight - weight);
      if (diff < minDiff) {
        minDiff = diff;
        closest = weight;
      }
    }

    return closest;
  };
</script>

{#snippet uploadFontFamilyItem()}
  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <Icon
      style={css.raw({ color: 'text.faint', transitionProperty: '[none]', _groupHover: { color: 'text.brand' } })}
      icon={PlusIcon}
      size={14}
    />
    <span class={css({ color: 'text.subtle', _groupHover: { color: 'text.brand' } })}>직접 업로드</span>
  </div>
{/snippet}

<SearchableDropdown
  style={css.raw({ width: '120px' })}
  disabled={!editor?.current.can().chain().setFontFamily(defaultValues.fontFamily).run()}
  extraItems={[
    {
      onclick: () => {
        if ($user.subscription) {
          uploadModalOpen = true;
        } else {
          planUpgradeOpen = true;
          mixpanel.track('open_plan_upgrade_modal', { via: 'font_family_upload' });
        }
      },
      content: uploadFontFamilyItem,
    },
  ]}
  getLabel={(value) => {
    const item = allFontFamilies.find((f) => f.value === value);
    return item?.label ?? '(알 수 없는 폰트)';
  }}
  items={allFontFamilies}
  label="폰트 패밀리"
  onEscape={() => editor?.current.commands.focus()}
  onchange={(fontFamilyValue, options) => {
    const fontWeight = editor?.current.getAttributes('text_style').fontWeight ?? defaultValues.fontWeight;
    const defaultWeight = getDefaultWeight(fontFamilyValue, fontWeight) ?? defaultValues.fontWeight;

    const chain = editor?.current.chain().setFontFamily(fontFamilyValue).setFontWeight(defaultWeight);
    if (options?.shouldFocus) {
      chain?.focus();
    }
    chain?.run();
  }}
  value={currentFontFamilyValue}
>
  {#snippet renderItem(item)}
    <div style:font-family={item.value}>{item.label}</div>
  {/snippet}
</SearchableDropdown>

<FontUploadModal userId={$user.id} bind:open={uploadModalOpen} />
<PlanUpgradeModal bind:open={planUpgradeOpen}>폰트 업로드 기능은 FULL ACCESS 플랜에서 사용할 수 있어요.</PlanUpgradeModal>
