<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { TypieError } from '@typie/lib/errors';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { HorizontalDivider, Icon, Modal, RingSpinner } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { SvelteMap } from 'svelte/reactivity';
  import GemIcon from '~icons/lucide/gem';
  import InfoIcon from '~icons/lucide/info';
  import TypeIcon from '~icons/lucide/type';
  import UploadIcon from '~icons/lucide/upload';
  import { values } from '$lib/editor/values';
  import { cache, unwrapError } from '$lib/graphql';
  import { uploadBlob } from '$lib/utils';
  import { graphql } from '$mearie';

  type Props = {
    open: boolean;
    userId: string;
  };

  let { open = $bindable(), userId }: Props = $props();

  const [persistBlobAsFont] = createMutation(
    graphql(`
      mutation FontUploadModal_PersistBlobAsFontLegacy_Mutation($input: PersistBlobAsFontLegacyInput!) {
        persistBlobAsFontLegacy(input: $input) {
          id
          subfamilyDisplayName
          weight

          family {
            id
            displayName
          }
        }
      }
    `),
  );

  let inflight = $state(false);
  let isDragging = $state(false);
  let uploadProgress = $state({ current: 0, total: 0 });

  const errorMap = {
    invalid_font_style: '폰트가 기울어져 있어요.',
  };

  const processFiles = async (files: FileList | null) => {
    if (!files || files.length === 0) {
      return;
    }

    inflight = true;
    uploadProgress = { current: 0, total: files.length };

    const results: {
      name: string;
      success: boolean;
      error?: string;
      familyId?: string;
      familyDisplayName?: string;
      weight?: number;
      subfamilyDisplayName?: string | null;
    }[] = [];

    // NOTE: 업로드 폭탄을 방지하기 위해 하나씩 업로드
    for (const file of files) {
      uploadProgress.current++;
      try {
        const path = await uploadBlob(file);
        const resp = await persistBlobAsFont({ input: { path } });
        cache.invalidate(
          { __typename: 'User', id: userId, $field: 'fontFamilies' },
          { __typename: 'User', id: userId, $field: 'documentFontFamilies' },
          { __typename: 'Document', $field: 'fontFamilies' },
        );

        results.push({
          name: resp.persistBlobAsFontLegacy.family.displayName,
          familyId: resp.persistBlobAsFontLegacy.family.id,
          familyDisplayName: resp.persistBlobAsFontLegacy.family.displayName,
          weight: resp.persistBlobAsFontLegacy.weight,
          subfamilyDisplayName: resp.persistBlobAsFontLegacy.subfamilyDisplayName,
          success: true,
        });
      } catch (err) {
        let errorMessage = '폰트 업로드에 실패했어요.';
        const error = unwrapError(err);
        if (error instanceof TypieError) {
          errorMessage = errorMap[error.code as never] ?? errorMessage;
        }
        results.push({
          name: file.name,
          success: false,
          error: errorMessage,
        });
      }
    }

    inflight = false;

    const successResults = results.filter((r) => r.success);
    const failureResults = results.filter((r) => !r.success);

    const getWeightLabel = (font: { weight: number; subfamilyDisplayName?: string | null }) => {
      return (
        values.fontWeight.find((f) => f.value === font.weight)?.label ??
        (font.subfamilyDisplayName ? `${font.subfamilyDisplayName} (${font.weight})` : String(font.weight))
      );
    };

    const formatFontGroups = (items: typeof successResults) => {
      const grouped = new SvelteMap<string, { displayName: string; subs: string[] }>();
      for (const r of items) {
        const id = r.familyId ?? '';
        if (!grouped.has(id)) grouped.set(id, { displayName: r.familyDisplayName ?? '', subs: [] });
        grouped.get(id)?.subs.push(getWeightLabel({ weight: r.weight ?? 0, subfamilyDisplayName: r.subfamilyDisplayName }));
      }
      return [...grouped.values()].map(({ displayName, subs }) => `• ${displayName} (${subs.join(', ')})`).join('\n');
    };

    if (successResults.length > 0 && failureResults.length === 0) {
      Dialog.alert({
        title: '폰트 업로드 완료',
        message: `${successResults.length}개의 폰트가 추가되었어요.\n\n${formatFontGroups(successResults)}\n\n업로드한 폰트는 설정 > 폰트 탭에서 관리할 수 있어요.`,
      });
      open = false;
    } else {
      const failureMessages = failureResults.map((r) => `• ${r.name}: ${r.error}`).join('\n');

      if (successResults.length === 0) {
        Dialog.alert({
          title: '폰트 업로드 실패',
          message: `${failureResults.length}개의 폰트 업로드에 실패했어요.\n\n${failureMessages}`,
        });
      } else {
        Dialog.alert({
          title: '폰트 업로드 일부 완료',
          message: `${successResults.length}개의 폰트가 추가되었어요.\n\n${formatFontGroups(successResults)}\n\n${failureResults.length}개의 폰트 업로드에 실패했어요.\n\n${failureMessages}\n\n업로드된 폰트는 설정 > 폰트 탭에서 관리할 수 있어요.`,
        });
      }
    }
  };

  const handleUpload = async () => {
    const picker = document.createElement('input');
    picker.type = 'file';
    picker.accept = '.ttf';
    picker.multiple = true;

    picker.addEventListener('change', async () => {
      await processFiles(picker.files);
    });

    picker.click();
  };

  const handleDragOver = (e: DragEvent) => {
    e.preventDefault();
    isDragging = true;
  };

  const handleDragLeave = (e: DragEvent) => {
    e.preventDefault();
    isDragging = false;
  };

  const handleDrop = async (e: DragEvent) => {
    e.preventDefault();
    isDragging = false;

    const files = e.dataTransfer?.files;
    if (!files) return;

    const fontFiles = [...files].filter((file) => /\.ttf$/i.test(file.name));

    if (fontFiles.length === 0) {
      Dialog.alert({
        title: '올바른 폰트 파일이 아니에요',
        message: 'TTF 파일만 업로드할 수 있어요.',
      });
      return;
    }

    const dataTransfer = new DataTransfer();
    fontFiles.forEach((file) => dataTransfer.items.add(file));
    await processFiles(dataTransfer.files);
  };
</script>

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

  <div class={flex({ flexDirection: 'column', gap: '18px', paddingX: '24px', paddingY: '16px' })}>
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
        <li>TTF 확장자를 가진 폰트 파일을 업로드할 수 있어요.</li>
        <li>기울어진 폰트는 업로드할 수 없어요.</li>
        <li>업로드된 폰트는 내 글이라면 어디서나 이용할 수 있어요.</li>
        <li>기존에 업로드한 폰트 목록은 설정 &gt; 폰트 탭에서 관리할 수 있어요.</li>
        <li>무료 폰트 혹은 이미 구매해 웹에서 사용할 수 있는 라이선스가 있는 폰트만 이용해 주세요.</li>
        <li>저작권에 위배되는 폰트는 삭제될 수 있어요.</li>
      </ul>
    </div>

    <div
      class={css({
        position: 'relative',
        borderRadius: '8px',
        border: '2px dashed',
        borderColor: isDragging ? 'accent.brand.default' : 'border.default',
        backgroundColor: isDragging ? 'accent.brand.subtle' : 'surface.default',
        padding: '24px',
        textAlign: 'center',
        transition: 'background',
        cursor: inflight ? 'default' : 'pointer',
        _hover: {
          backgroundColor: isDragging ? 'accent.brand.subtle' : 'surface.subtle',
        },
      })}
      aria-busy={inflight}
      aria-disabled={inflight}
      onclick={inflight ? undefined : handleUpload}
      ondragleave={handleDragLeave}
      ondragover={handleDragOver}
      ondrop={inflight ? undefined : handleDrop}
      onkeydown={(e) => {
        if (!inflight && (e.key === 'Enter' || e.key === ' ')) {
          e.preventDefault();
          handleUpload();
        }
      }}
      role="button"
      tabindex={inflight ? -1 : 0}
    >
      <div class={flex({ flexDirection: 'column', gap: '12px', alignItems: 'center' })}>
        {#if inflight}
          <RingSpinner style={css.raw({ color: 'text.subtle', size: '24px' })} />
          <div class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.subtle' })}>
            폰트 업로드 중... ({uploadProgress.current}/{uploadProgress.total})
          </div>
        {:else}
          <Icon style={css.raw({ color: isDragging ? 'text.brand' : 'text.subtle' })} icon={UploadIcon} size={24} />
          <div class={flex({ flexDirection: 'column', gap: '4px' })}>
            <div class={css({ fontSize: '14px', fontWeight: 'medium', color: isDragging ? 'text.brand' : 'text.subtle' })}>
              클릭하거나 파일을 드래그해서 업로드
            </div>
            <div class={css({ fontSize: '12px', color: 'text.subtle' })}>TTF 파일 (여러 개 가능)</div>
          </div>
        {/if}
      </div>
    </div>
  </div>
</Modal>
