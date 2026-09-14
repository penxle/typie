<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { DocumentContentRating, EntityVisibility } from '@typie/lib/enums';
  import { TypieError } from '@typie/lib/errors';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { HorizontalDivider, Icon, Select, Switch } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import BanIcon from '~icons/lucide/ban';
  import BlendIcon from '~icons/lucide/blend';
  import CheckIcon from '~icons/lucide/check';
  import Dice5Icon from '~icons/lucide/dice-5';
  import GlobeIcon from '~icons/lucide/globe';
  import IdCardIcon from '~icons/lucide/id-card';
  import LinkIcon from '~icons/lucide/link';
  import LockIcon from '~icons/lucide/lock';
  import LockKeyholeIcon from '~icons/lucide/lock-keyhole';
  import ShieldIcon from '~icons/lucide/shield';
  import SmileIcon from '~icons/lucide/smile';
  import UsersRoundIcon from '~icons/lucide/users-round';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import type { DashboardLayout_Share_Document_document$key } from '$mearie';

  type Props = {
    documents$key: DashboardLayout_Share_Document_document$key[];
  };

  let { documents$key }: Props = $props();

  const documents = createFragment(
    graphql(`
      fragment DashboardLayout_Share_Document_document on Document {
        id
        title
        password
        documentContentRating: contentRating
        allowReaction
        protectContent

        entity {
          id
          url
          visibility
        }

        publication {
          id
          state
          url
        }
      }
    `),
    () => documents$key,
  );

  const isSingleDocument = $derived(documents.data.length === 1);
  const documentIds = $derived(documents.data.map((d) => d.id));

  const isPublished = $derived(documents.data.some((d) => d.publication?.state === 'PUBLISHED'));
  const isPublic = $derived(documents.data.some((d) => d.entity.visibility === EntityVisibility.PUBLIC));

  const visibilityItems = $derived([
    ...(isPublished || isPublic ? [{ icon: GlobeIcon, label: '발행됨', value: EntityVisibility.PUBLIC }] : []),
    {
      icon: LinkIcon,
      label: '링크가 있는 사람',
      description: '링크가 있는 누구나 볼 수 있어요.',
      value: EntityVisibility.UNLISTED,
    },
    {
      icon: LockIcon,
      label: '비공개',
      description: '나만 볼 수 있어요.',
      value: EntityVisibility.PRIVATE,
    },
  ]);

  type UpdateDocumentsOptionInput = {
    visibility?: EntityVisibility | null;
    password?: string | null;
    contentRating?: DocumentContentRating | null;
    allowReaction?: boolean | null;
    protectContent?: boolean | null;
  };

  const isPrivateVisibilityOnlyInput = (input: UpdateDocumentsOptionInput): boolean =>
    input.visibility === EntityVisibility.PRIVATE &&
    input.password == null &&
    input.contentRating == null &&
    input.allowReaction == null &&
    input.protectContent == null;

  const [updateDocumentsOption] = createMutation(
    graphql(`
      mutation DashboardLayout_Share_Document_UpdateDocumentsOption_Mutation($input: UpdateDocumentsOptionInput!) {
        updateDocumentsOption(input: $input) {
          id
          password
          documentContentRating: contentRating
          allowReaction
          protectContent

          entity {
            id
            visibility
          }
        }
      }
    `),
  );

  let copied = $state(false);
  let timer: NodeJS.Timeout | undefined;

  let isRolling = $state(false);

  const visibilityIndeterminate = $derived(
    documents.data.length > 1 && documents.data.some((d) => d.entity.visibility !== documents.data[0].entity.visibility),
  );

  const form = createForm({
    schema: z.object({
      visibility: z.nativeEnum(EntityVisibility),
      hasPassword: z.boolean(),
      password: z.string().trim().nullish(),
      documentContentRating: z.nativeEnum(DocumentContentRating),
      allowReaction: z.boolean(),
      protectContent: z.boolean(),
    }),
    submitOn: 'change',
    onSubmit: async (data) => {
      if (documents.data.length === 0) return;

      const dirtyFields = form.getDirtyFields();
      const updateData: {
        documentIds: string[];
        visibility?: EntityVisibility;
        contentRating?: DocumentContentRating;
        allowReaction?: boolean;
        protectContent?: boolean;
        password?: string | null;
      } = { documentIds };

      if ('visibility' in dirtyFields) updateData.visibility = data.visibility;
      if ('documentContentRating' in dirtyFields) updateData.contentRating = data.documentContentRating;
      if ('allowReaction' in dirtyFields) updateData.allowReaction = data.allowReaction;
      if ('protectContent' in dirtyFields) updateData.protectContent = data.protectContent;
      if ('hasPassword' in dirtyFields || 'password' in dirtyFields) updateData.password = data.hasPassword ? data.password : null;

      if (Object.keys(updateData).length > 1) {
        if (!isPrivateVisibilityOnlyInput(updateData) && !SubscribeModal.gate('share_document')) {
          return;
        }

        await updateDocumentsOption({ input: updateData });

        mixpanel.track('update_document_option', {
          ...updateData,
          hasPassword: data.hasPassword,
          count: documents.data.length,
        });
      }
    },
    onError: (error) => {
      if (error instanceof TypieError) {
        const message = publicationErrorMessage(error.code);
        Toast.error(message);
        throw new FormError('visibility', message);
      }
    },
    defaultValues: {
      visibility: documents.data[0].entity.visibility,
      hasPassword: documents.data[0].password !== null,
      password:
        documents.data.length > 1 && documents.data.some((d) => d.password !== documents.data[0].password)
          ? null
          : documents.data[0].password,
      documentContentRating: documents.data[0].documentContentRating,
      allowReaction: documents.data[0].allowReaction,
      protectContent: documents.data[0].protectContent,
    },
  });

  $effect(() => {
    void form;
  });

  $effect(() => {
    return () => {
      if (timer) {
        clearTimeout(timer);
      }
    };
  });

  const generateRandomPassword = () => {
    isRolling = true;

    const digits = '0123456789';
    let password = '';
    for (let i = 0; i < 4; i++) {
      password += digits.charAt(Math.floor(Math.random() * digits.length));
    }
    form.fields.password = password;

    setTimeout(() => {
      isRolling = false;
    }, 500);
  };

  const handleCopyLink = () => {
    if (documents.data.length === 0) return;

    const text = documents.data.map((d) => d.entity.url).join('\n');
    if (!text) return;

    navigator.clipboard.writeText(text);
    mixpanel.track('copy_document_share_url', { tab: 'view', count: documents.data.length });

    if (timer) {
      clearTimeout(timer);
    }

    copied = true;
    timer = setTimeout(() => (copied = false), 2000);
  };
</script>

<div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px', paddingX: '16px', paddingY: '12px' })}>
  <div class={flex({ gap: '[0.5ch]', fontSize: '12px', fontWeight: 'medium' })}>
    <span class={css({ wordBreak: 'break-all', lineClamp: '1', fontWeight: 'semibold' })}>
      {isSingleDocument ? documents.data[0].title : `${documents.data.length}개의 문서`}
    </span>
    <span class={css({ flexShrink: '0' })}>공유 및 발행</span>
  </div>
  <button
    class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}
    onclick={handleCopyLink}
    type="button"
    use:tooltip={{
      message: visibilityIndeterminate
        ? null
        : form.fields.visibility === EntityVisibility.PRIVATE
          ? '지금은 링크가 있어도 나만 볼 수 있어요'
          : '링크가 있는 누구나 문서를 볼 수 있어요',
      placement: 'top',
      keepOnClick: true,
    }}
  >
    {#if copied}
      <Icon style={css.raw({ color: 'accent.default' })} icon={CheckIcon} size={12} />
      <div class={css({ fontSize: '12px', color: 'text.default' })}>복사되었어요</div>
    {:else}
      <Icon style={css.raw({ color: 'text.default' })} icon={LinkIcon} size={12} />
      <div class={css({ fontSize: '12px', color: 'text.default' })}>
        {isSingleDocument ? '링크 복사' : '링크 모두 복사'}
      </div>
    {/if}
  </button>
</div>

<HorizontalDivider />

<div class={flex({ flexDirection: 'column', gap: '16px', paddingX: '16px', paddingTop: '16px', paddingBottom: '24px' })}>
  <div class={flex({ flexDirection: 'column', gap: '12px' })}>
    <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>문서 조회 권한</div>

    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'text.muted' })} icon={BlendIcon} size={14} />
        <div class={css({ fontSize: '12px', color: 'text.muted' })}>공개 범위</div>
      </div>

      <div use:tooltip={{ message: isPublished ? '공개 방식을 바꾸려면 먼저 발행을 취소해야 해요.' : null, placement: 'top' }}>
        <Select
          disabled={isPublished}
          items={visibilityItems}
          values={documents.data.map((d) => d.entity.visibility)}
          bind:value={form.fields.visibility}
        />
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.muted' })} icon={LockKeyholeIcon} />
          <div class={css({ fontSize: '12px', color: 'text.muted' })}>비밀번호 보호</div>
        </div>

        <Switch values={documents.data.map((d) => d.password !== null)} bind:checked={form.fields.hasPassword} />
      </div>

      {#if form.fields.hasPassword}
        <div class={flex({ position: 'relative' })}>
          <input
            class={css({
              borderWidth: '1px',
              borderRadius: '6px',
              paddingLeft: '12px',
              paddingRight: '32px',
              width: 'full',
              height: '32px',
              fontFamily: 'mono',
              fontSize: '12px',
              color: 'text.default',
            })}
            autocomplete="off"
            data-1p-ignore
            placeholder="비밀번호 입력"
            type="text"
            bind:value={form.fields.password}
          />

          <button
            class={center({
              position: 'absolute',
              top: '1/2',
              right: '8px',
              size: '20px',
              color: 'text.muted',
              userSelect: 'none',
              translate: 'auto',
              translateY: '-1/2',
              _hover: { color: 'text.default' },
            })}
            onclick={generateRandomPassword}
            type="button"
            use:tooltip={{
              message: '4자리 랜덤 비밀번호 생성',
              placement: 'bottom',
            }}
          >
            <Icon
              class={css({
                animation: isRolling ? 'diceRoll 0.5s cubic-bezier(0.4, 0.0, 0.2, 1)' : 'none',
                transformOrigin: 'center',
              })}
              icon={Dice5Icon}
              size={16}
            />
          </button>
        </div>
      {/if}
    </div>

    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'text.muted' })} icon={IdCardIcon} />
        <div class={css({ fontSize: '12px', color: 'text.muted' })}>연령 제한</div>
      </div>

      <Select
        items={[
          { label: '없음', value: DocumentContentRating.ALL },
          { label: '15세', value: DocumentContentRating.R15 },
          { label: '성인', value: DocumentContentRating.R19 },
        ]}
        values={documents.data.map((d) => d.documentContentRating)}
        bind:value={form.fields.documentContentRating}
      />
    </div>
  </div>

  <div class={flex({ flexDirection: 'column', gap: '12px' })}>
    <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>문서 상호작용</div>

    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'text.muted' })} icon={SmileIcon} />
        <div class={css({ fontSize: '12px', color: 'text.muted' })}>이모지 반응</div>
      </div>

      <Select
        items={[
          { icon: UsersRoundIcon, label: '누구나', value: true },
          { icon: BanIcon, label: '비허용', value: false },
        ]}
        values={documents.data.map((d) => d.allowReaction)}
        bind:value={form.fields.allowReaction}
      />
    </div>
  </div>

  <div class={flex({ flexDirection: 'column', gap: '12px' })}>
    <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>문서 보호</div>

    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'text.muted' })} icon={ShieldIcon} />
        <div class={flex({ flexDirection: 'column' })}>
          <div class={css({ fontSize: '12px', color: 'text.muted' })}>내용 보호</div>
          <p class={css({ fontSize: '10px', color: 'text.muted' })}>우클릭, 복사 및 다운로드 제한</p>
        </div>
      </div>

      <Switch values={documents.data.map((d) => d.protectContent)} bind:checked={form.fields.protectContent} />
    </div>
  </div>
</div>
