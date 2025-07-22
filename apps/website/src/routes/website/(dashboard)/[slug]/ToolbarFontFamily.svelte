<script lang="ts">
  import { TypieError } from '@/errors';
  import GemIcon from '~icons/lucide/gem';
  import InfoIcon from '~icons/lucide/info';
  import PlusIcon from '~icons/lucide/plus';
  import TypeIcon from '~icons/lucide/type';
  import { fragment, graphql } from '$graphql';
  import { tooltip } from '$lib/actions';
  import { Button, HorizontalDivider, Icon, Modal } from '$lib/components';
  import { Dialog } from '$lib/notification';
  import { defaultValues, values } from '$lib/tiptap/values';
  import { uploadBlob } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import PlanUpgradeModal from '../PlanUpgradeModal.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarDropdownMenu from './ToolbarDropdownMenu.svelte';
  import ToolbarDropdownMenuItem from './ToolbarDropdownMenuItem.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Editor_Toolbar_FontFamily_site, Optional } from '$graphql';
  import type { Ref } from '$lib/utils';

  type Props = {
    $site: Optional<Editor_Toolbar_FontFamily_site>;
    editor?: Ref<Editor>;
  };

  let { $site: _site, editor }: Props = $props();

  let planUpgradeOpen = $state(false);

  const site = fragment(
    _site,
    graphql(`
      fragment Editor_Toolbar_FontFamily_site on Site {
        id

        fonts {
          id
          name
        }

        user {
          id

          subscription {
            id
          }
        }
      }
    `),
  );

  const persistBlobAsFont = graphql(`
    mutation Editor_Toolbar_FontFamily_PersistBlobAsFont_Mutation($input: PersistBlobAsFontInput!) {
      persistBlobAsFont(input: $input) {
        id
        name
      }
    }
  `);

  const addSiteFont = graphql(`
    mutation Editor_Toolbar_FontFamily_AddSiteFont_Mutation($input: AddSiteFontInput!) {
      addSiteFont(input: $input) {
        id

        fonts {
          id
          name
        }
      }
    }
  `);

  let open = $state(false);
  let inflight = $state(false);

  const errorMap = {
    invalid_font_weight: '폰트가 너무 얇거나 두꺼워요.',
    invalid_font_style: '폰트가 기울어져 있어요.',
  };

  const handleUpload = async () => {
    const picker = document.createElement('input');
    picker.type = 'file';
    picker.accept = '.ttf,.otf';

    picker.addEventListener('change', async () => {
      const file = picker.files?.[0];
      if (!file || !$site) {
        return;
      }

      inflight = true;
      try {
        const path = await uploadBlob(file);
        const resp = await persistBlobAsFont({ path });
        await addSiteFont({ siteId: $site.id, fontId: resp.id });

        open = false;
        Dialog.alert({
          title: '폰트 업로드 완료',
          message: `"${resp.name}" 폰트가 추가되었어요. 업로드한 폰트는 설정 > 사이트 탭에서 관리할 수 있어요.`,
        });
      } catch (err) {
        if (err instanceof TypieError) {
          Dialog.alert({
            title: '폰트 업로드 실패',
            message: errorMap[err.code as never] ?? '폰트 업로드에 실패했어요.',
          });
        } else {
          Dialog.alert({
            title: '폰트 업로드 실패',
            message: '폰트 업로드에 실패했어요.',
          });
        }
      } finally {
        inflight = false;
        open = false;
      }
    });

    picker.click();
  };
</script>

<ToolbarDropdownButton
  style={css.raw({ width: '120px' })}
  chevron
  disabled={!editor?.current.can().chain().focus().setFontFamily(defaultValues.fontFamily).run()}
  label="글씨 서체"
  size="small"
>
  {#snippet anchor()}
    <div class={css({ flexGrow: '1', fontSize: '14px', color: 'text.subtle', lineClamp: '1' })}>
      {values.fontFamily.find(({ value }) => value === (editor?.current.getAttributes('text_style').fontFamily ?? defaultValues.fontFamily))
        ?.label ??
        $site?.fonts.find(({ id }) => id === (editor?.current.getAttributes('text_style').fontFamily ?? defaultValues.fontFamily))?.name}
    </div>
  {/snippet}

  {#snippet floating({ close })}
    <ToolbarDropdownMenu>
      {#each values.fontFamily as { label, value } (value)}
        <ToolbarDropdownMenuItem
          active={(editor?.current.getAttributes('text_style').fontFamily ?? defaultValues.fontFamily) === value}
          onclick={() => {
            editor?.current.chain().focus().setFontFamily(value).run();
            close();
          }}
        >
          <div style:font-family={value}>{label}</div>
        </ToolbarDropdownMenuItem>
      {/each}

      {#if $site?.user.subscription}
        {#each $site.fonts as { id, name } (id)}
          <ToolbarDropdownMenuItem
            active={(editor?.current.getAttributes('text_style').fontFamily ?? defaultValues.fontFamily) === name}
            onclick={() => {
              editor?.current
                .chain()
                .focus()
                .setFontFamily(id as never)
                .run();
              close();
            }}
          >
            <div style:font-family={id}>{name}</div>
          </ToolbarDropdownMenuItem>
        {/each}
      {/if}
      <ToolbarDropdownMenuItem
        onclick={() => {
          open = true;
          close();
        }}
      >
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          <Icon
            style={css.raw({ color: 'text.faint', transitionProperty: 'none', _groupHover: { color: 'text.brand' } })}
            icon={PlusIcon}
            size={14}
          />
          <span class={css({ color: 'text.subtle', _groupHover: { color: 'text.brand' } })}>직접 업로드</span>
        </div>
      </ToolbarDropdownMenuItem>
    </ToolbarDropdownMenu>
  {/snippet}
</ToolbarDropdownButton>

<PlanUpgradeModal bind:open={planUpgradeOpen} />

<Modal style={css.raw({ maxWidth: '400px' })} bind:open>
  <div class={center({ gap: '8px', padding: '12px' })}>
    <div class={center({ gap: '4px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} size={14} />
      <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.faint' })}>폰트 업로드하기</span>
    </div>

    <div
      class={center({
        gap: '4px',
        borderRadius: 'full',
        paddingX: '8px',
        paddingY: '2px',
        backgroundColor: 'accent.brand.subtle',
        userSelect: 'none',
      })}
      use:tooltip={{ message: 'FULL ACCESS 전용 기능이에요', placement: 'top', delay: 0 }}
    >
      <Icon style={css.raw({ color: 'text.brand' })} icon={GemIcon} size={12} />
      <span class={css({ fontSize: '11px', fontWeight: 'bold', color: 'text.brand' })}>FULL</span>
    </div>
  </div>

  <HorizontalDivider />

  <div class={flex({ flexDirection: 'column', gap: '24px', paddingX: '24px', paddingY: '16px' })}>
    <div
      class={flex({
        flexDirection: 'column',
        gap: '8px',
        borderRadius: '4px',
        fontSize: '14px',
        backgroundColor: 'surface.muted',
        padding: '12px',
      })}
    >
      <div class={center({ gap: '4px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={InfoIcon} size={12} />
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.faint' })}>이용 안내</span>
      </div>

      <ul class={css({ listStyle: 'disc', paddingLeft: '20px', fontSize: '13px', color: 'text.faint' })}>
        <li>TTF, OTF 확장자를 가진 폰트 파일을 업로드할 수 있어요.</li>
        <li>너무 얇거나 너무 두꺼운 폰트, 기울어진 폰트는 업로드할 수 없어요.</li>
        <li>업로드된 폰트는 내 글이라면 어디서나 이용할 수 있어요.</li>
        <li>기존에 업로드한 폰트 목록은 설정 &gt; 사이트 탭에서 관리할 수 있어요.</li>
        <li>무료 폰트 혹은 이미 구매해 웹에서 사용할 수 있는 라이선스가 있는 폰트만 이용해 주세요.</li>
        <li>저작권에 위배되는 폰트는 삭제될 수 있어요.</li>
      </ul>
    </div>

    {#if $site?.user.subscription}
      <Button loading={inflight} onclick={handleUpload}>파일 선택</Button>
    {:else}
      <Button onclick={() => (planUpgradeOpen = true)}>파일 선택</Button>
    {/if}
  </div>
</Modal>
