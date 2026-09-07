<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { DocumentContentRating } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem, Switch } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import BanIcon from '~icons/lucide/ban';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import EyeIcon from '~icons/lucide/eye';
  import EyeOffIcon from '~icons/lucide/eye-off';
  import IdCardIcon from '~icons/lucide/id-card';
  import LockKeyholeIcon from '~icons/lucide/lock-keyhole';
  import PencilIcon from '~icons/lucide/pencil';
  import ShieldIcon from '~icons/lucide/shield';
  import SmileIcon from '~icons/lucide/smile';
  import UsersRoundIcon from '~icons/lucide/users-round';
  import XIcon from '~icons/lucide/x';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import PropertyRow from './PropertyRow.svelte';
  import { menuItemStyle, menuListStyle, propertyChevronStyle, propertyTriggerStyle } from './publish-styles';
  import type { DashboardLayout_Share_ReadingSettings_document$key } from '$mearie';

  type Props = {
    document$key: DashboardLayout_Share_ReadingSettings_document$key;
  };

  let { document$key }: Props = $props();

  const document = createFragment(
    graphql(`
      fragment DashboardLayout_Share_ReadingSettings_document on Document {
        id
        password
        contentRating
        allowReaction
        protectContent
      }
    `),
    () => document$key,
  );

  const [updateDocumentsOption] = createMutation(
    graphql(`
      mutation DashboardLayout_Share_ReadingSettings_UpdateDocumentsOption_Mutation($input: UpdateDocumentsOptionInput!) {
        updateDocumentsOption(input: $input) {
          id
          password
          contentRating
          allowReaction
          protectContent
        }
      }
    `),
  );

  type Option = {
    password?: string | null;
    contentRating?: DocumentContentRating;
    allowReaction?: boolean;
    protectContent?: boolean;
  };

  const RATING_ITEMS = [
    { label: '없음', value: DocumentContentRating.ALL },
    { label: '15세', value: DocumentContentRating.R15 },
    { label: '성인', value: DocumentContentRating.R19 },
  ];

  const REACTION_ITEMS = [
    { icon: UsersRoundIcon, label: '누구나', value: true },
    { icon: BanIcon, label: '비허용', value: false },
  ];

  let protectContent = $state(document.data.protectContent);

  const password = $derived(document.data.password);

  let editing = $state(false);
  let draft = $state('');
  let revealed = $state(false);
  let saving = $state(false);
  let passwordEl = $state<HTMLInputElement>();
  let passwordFieldEl = $state<HTMLElement>();

  const update = async (option: Option): Promise<boolean> => {
    if (!SubscribeModal.gate('share_document')) return false;

    try {
      await updateDocumentsOption({ input: { documentIds: [document.data.id], ...option } });
      mixpanel.track('update_document_option', { ...option, via: 'share_modal' });
      return true;
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      return false;
    }
  };

  $effect(() => {
    if (!editing) return;

    const el = passwordEl;
    if (!el) return;

    const frame = requestAnimationFrame(() => el.select());
    return () => cancelAnimationFrame(frame);
  });

  const startEditing = () => {
    draft = password ?? '';
    editing = true;
  };

  const cancelEditing = () => {
    editing = false;
    draft = '';
  };

  $effect(() => {
    if (!editing) return;

    const handler = (event: PointerEvent) => {
      const el = passwordFieldEl;
      if (!el || el.contains(event.target as Node)) return;

      cancelEditing();
    };

    window.addEventListener('pointerdown', handler);
    return () => window.removeEventListener('pointerdown', handler);
  });

  const savePassword = async () => {
    const next = draft.trim();
    if (saving || next.length === 0) return;

    saving = true;
    const saved = await update({ password: next });
    saving = false;

    if (saved) {
      editing = false;
      revealed = false;
    }
  };

  const clearPassword = async () => {
    if (saving) return;

    saving = true;
    const saved = await update({ password: null });
    saving = false;

    if (saved) {
      editing = false;
      revealed = false;
      draft = '';
    }
  };

  const handlePasswordKeydown = (event: KeyboardEvent) => {
    if (event.isComposing) return;

    if (event.key === 'Enter') {
      event.preventDefault();
      void savePassword();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      cancelEditing();
    }
  };

  const passwordIconButtonStyle = css.raw({
    flexShrink: '0',
    size: '24px',
    borderRadius: '4px',
    color: 'text.muted',
    transition: 'common',
    _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
    _disabled: { opacity: '30', cursor: 'default', _hover: { color: 'text.muted', backgroundColor: 'transparent' } },
  });

  const passwordValueStyle = css.raw({
    fontFamily: 'mono',
    fontSize: '13px',
    color: 'text.default',
  });
</script>

{#snippet menuCheck(selected: boolean)}
  <span class={css({ flexShrink: '0', width: '14px', color: 'accent.default' })}>
    {#if selected}
      <Icon icon={CheckIcon} size={14} />
    {/if}
  </span>
{/snippet}

<div class={flex({ flexDirection: 'column', gap: '1px', marginX: '-8px' })}>
  <PropertyRow icon={LockKeyholeIcon} label="비밀번호 보호">
    {#if editing}
      <span
        bind:this={passwordFieldEl}
        class={flex({
          alignItems: 'center',
          width: '220px',
          height: '30px',
          paddingLeft: '10px',
          paddingRight: '3px',
          borderWidth: '1px',
          borderColor: 'accent.default',
          borderRadius: '6px',
          backgroundColor: 'surface.default',
        })}
      >
        <input
          bind:this={passwordEl}
          class={css(passwordValueStyle, {
            flex: '1',
            minWidth: '0',
            height: 'full',
            _placeholder: { fontFamily: 'mono', color: 'text.hint' },
          })}
          aria-label="비밀번호"
          autocomplete="off"
          data-1p-ignore
          onkeydown={handlePasswordKeydown}
          placeholder="비밀번호 입력"
          type="text"
          bind:value={draft}
        />

        <button
          class={center(passwordIconButtonStyle)}
          aria-label={password ? '수정 취소' : '설정 취소'}
          onclick={cancelEditing}
          type="button"
          use:tooltip={{ message: '취소', placement: 'top' }}
        >
          <Icon icon={XIcon} size={14} />
        </button>

        <button
          class={center(passwordIconButtonStyle)}
          aria-label="비밀번호 저장"
          disabled={draft.trim().length === 0 || saving}
          onclick={() => savePassword()}
          type="button"
          use:tooltip={{ message: '저장', placement: 'top' }}
        >
          <Icon icon={CheckIcon} size={14} />
        </button>
      </span>
    {:else if password}
      <span class={css(passwordValueStyle, revealed ? {} : { letterSpacing: '[0.12em]' })}>
        {revealed ? password : '•'.repeat(password.length)}
      </span>

      <button
        class={center(passwordIconButtonStyle)}
        aria-label={revealed ? '비밀번호 가리기' : '비밀번호 보기'}
        onclick={() => (revealed = !revealed)}
        type="button"
        use:tooltip={{ message: revealed ? '가리기' : '보기', placement: 'top', keepOnClick: true }}
      >
        <Icon icon={revealed ? EyeOffIcon : EyeIcon} size={14} />
      </button>

      <button
        class={center(passwordIconButtonStyle)}
        aria-label="비밀번호 변경"
        data-primary
        onclick={startEditing}
        type="button"
        use:tooltip={{ message: '변경', placement: 'top' }}
      >
        <Icon icon={PencilIcon} size={14} />
      </button>

      <button
        class={center(css.raw(passwordIconButtonStyle, { _hover: { color: 'danger.default', backgroundColor: 'surface.hover' } }))}
        aria-label="비밀번호 해제"
        disabled={saving}
        onclick={() => clearPassword()}
        type="button"
        use:tooltip={{ message: '해제', placement: 'top' }}
      >
        <Icon icon={XIcon} size={14} />
      </button>
    {:else}
      <button class={css(propertyTriggerStyle, { color: 'text.hint' })} data-primary onclick={startEditing} type="button">없음</button>
    {/if}
  </PropertyRow>

  <PropertyRow icon={IdCardIcon} label="연령 제한">
    <Menu
      style={propertyTriggerStyle}
      buttonAriaLabel="연령 제한"
      listStyle={css.raw(menuListStyle, { minWidth: '160px' })}
      offset={4}
      placement="bottom-start"
    >
      {#snippet button()}
        <span>{RATING_ITEMS.find((item) => item.value === document.data.contentRating)?.label}</span>
        <Icon style={propertyChevronStyle} icon={ChevronDownIcon} size={14} />
      {/snippet}

      {#each RATING_ITEMS as item (item.value)}
        <MenuItem style={menuItemStyle} onclick={() => update({ contentRating: item.value })}>
          <span class={css({ flex: '1' })}>{item.label}</span>

          {#snippet suffix()}
            {@render menuCheck(document.data.contentRating === item.value)}
          {/snippet}
        </MenuItem>
      {/each}
    </Menu>
  </PropertyRow>

  <PropertyRow icon={SmileIcon} label="이모지 반응">
    <Menu
      style={propertyTriggerStyle}
      buttonAriaLabel="이모지 반응"
      listStyle={css.raw(menuListStyle, { minWidth: '160px' })}
      offset={4}
      placement="bottom-start"
    >
      {#snippet button()}
        {@const current = REACTION_ITEMS.find((item) => item.value === document.data.allowReaction)}
        {#if current}
          <Icon style={css.raw({ flexShrink: '0', color: 'text.muted' })} icon={current.icon} size={14} />
          <span>{current.label}</span>
        {/if}
        <Icon style={propertyChevronStyle} icon={ChevronDownIcon} size={14} />
      {/snippet}

      {#each REACTION_ITEMS as item (item.label)}
        <MenuItem style={menuItemStyle} icon={item.icon} onclick={() => update({ allowReaction: item.value })}>
          <span class={css({ flex: '1' })}>{item.label}</span>

          {#snippet suffix()}
            {@render menuCheck(document.data.allowReaction === item.value)}
          {/snippet}
        </MenuItem>
      {/each}
    </Menu>
  </PropertyRow>

  <PropertyRow hint="우클릭, 복사 및 다운로드 제한" icon={ShieldIcon} label="내용 보호">
    <Switch onchange={() => update({ protectContent })} bind:checked={protectContent} />
  </PropertyRow>
</div>
