<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { onMount } from 'svelte';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { getCommentContext } from './context.svelte';

  type Props = {
    placeholder?: string;
    autofocus?: boolean;
    onsubmit: (content: string) => void | Promise<void>;
    oncancel?: () => void;
    ondirty?: (dirty: boolean) => void;
  };
  let { placeholder = '코멘트 입력...', autofocus = false, onsubmit, oncancel, ondirty }: Props = $props();

  const comments = getCommentContext();

  const user = createFragment(
    graphql(`
      fragment CommentComposerV2_user on User {
        id
        name
        avatar {
          id
          ...Img_image
        }
      }
    `),
    () => comments.meUser,
  );

  let value = $state('');
  let pending = $state(false);
  let textareaEl = $state<HTMLTextAreaElement>();
  const hasContent = $derived(value.length > 0);
  const hasText = $derived(value.trim().length > 0);

  onMount(() => {
    if (!autofocus) return;
    requestAnimationFrame(() => textareaEl?.focus());
  });

  $effect(() => {
    ondirty?.(hasText);
    return () => ondirty?.(false);
  });

  const submit = async () => {
    if (!hasText || pending) return;
    pending = true;
    try {
      await onsubmit(value.trim());
      value = '';
    } catch {
      // ignore
    } finally {
      pending = false;
    }
  };

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      void submit();
    } else if (e.key === 'Escape') {
      e.stopPropagation();
      if (value.trim() === '') {
        oncancel?.();
      } else {
        Dialog.confirm({
          title: '작성 중인 내용 삭제',
          message: '작성 중인 내용을 지우시겠어요?',
          action: 'danger',
          actionLabel: '지우기',
          actionHandler: () => {
            value = '';
          },
        });
      }
    }
  };
</script>

<div class={css({ display: 'flex', gap: '8px', alignItems: 'flex-start', paddingY: '8px', paddingLeft: '10px', paddingRight: '8px' })}>
  {#if user.data?.avatar}
    <Img
      style={css.raw({ size: '24px', borderRadius: 'full', flexShrink: '0', marginTop: '1px' })}
      alt={user.data.name}
      image$key={user.data.avatar}
      size={24}
    />
  {/if}
  <div class={css({ position: 'relative', display: 'flex', flexGrow: '1', minWidth: '0' })}>
    <textarea
      bind:this={textareaEl}
      style:padding-right={hasContent ? '10px' : '40px'}
      style:padding-bottom={hasContent ? '40px' : '8px'}
      class={css({
        width: 'full',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '6px',
        paddingLeft: '10px',
        paddingTop: '8px',
        fontSize: '13px',
        lineHeight: '[1.4]',
        color: 'text.default',
        backgroundColor: 'surface.subtle',
        resize: 'none',
        minHeight: '36px',
        maxHeight: '120px',
        outline: 'none',
        transition: 'colors',
        _focus: { borderColor: 'accent.brand.default', backgroundColor: 'surface.default' },
      })}
      onkeydown={handleKeydown}
      {placeholder}
      rows={1}
      bind:value
      use:autosize
    ></textarea>
    <button
      style:top={hasContent ? 'auto' : '50%'}
      style:bottom={hasContent ? '6px' : 'auto'}
      style:transform={hasContent ? undefined : 'translateY(-50%)'}
      class={css(
        {
          position: 'absolute',
          right: '6px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          size: '22px',
          borderRadius: 'full',
          transition: 'common',
        },
        hasText && !pending
          ? {
              cursor: 'pointer',
              backgroundColor: 'accent.brand.default',
              color: 'text.bright',
              _hover: { backgroundColor: 'accent.brand.hover' },
              _active: { backgroundColor: 'accent.brand.active' },
            }
          : { cursor: 'default', backgroundColor: 'surface.muted', color: 'text.disabled' },
      )}
      disabled={!hasText || pending}
      onclick={() => void submit()}
      type="button"
      use:tooltip={{ message: '보내기', placement: 'bottom' }}
    >
      {#if pending}
        <RingSpinner style={css.raw({ size: '12px' })} />
      {:else}
        <Icon icon={ArrowUpIcon} size={12} />
      {/if}
    </button>
  </div>
</div>
