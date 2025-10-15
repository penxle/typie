<script lang="ts">
  import { cache } from '@typie/sark/internal';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { HorizontalDivider, Icon, Modal, RingSpinner } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { TypieError } from '@/errors';
  import GemIcon from '~icons/lucide/gem';
  import InfoIcon from '~icons/lucide/info';
  import TypeIcon from '~icons/lucide/type';
  import UploadIcon from '~icons/lucide/upload';
  import { graphql } from '$graphql';
  import { uploadBlob } from '$lib/utils';

  type Props = {
    open: boolean;
    userId: string;
  };

  let { open = $bindable(), userId }: Props = $props();

  const persistBlobAsFont = graphql(`
    mutation FontUploadModal_PersistBlobAsFont_Mutation($input: PersistBlobAsFontInput!) {
      persistBlobAsFont(input: $input) {
        id
        name
      }
    }
  `);

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

    const results: { name: string; success: boolean; error?: string }[] = [];

    // NOTE: 업로드 폭탄을 방지하기 위해 하나씩 업로드
    for (const file of files) {
      uploadProgress.current++;
      try {
        const path = await uploadBlob(file);
        const resp = await persistBlobAsFont({ path });
        cache.invalidate({ __typename: 'User', id: userId, field: 'fontFamilies' });

        results.push({ name: resp.name, success: true });
      } catch (err) {
        let errorMessage = '폰트 업로드에 실패했어요.';
        if (err instanceof TypieError) {
          errorMessage = errorMap[err.code as never] ?? errorMessage;
        }
        results.push({
          name: file.name,
          success: false,
          error: errorMessage,
        });
      }
    }

    inflight = false;

    const successCount = results.filter((r) => r.success).length;
    const failureCount = results.filter((r) => !r.success).length;

    if (successCount > 0 && failureCount === 0) {
      if (successCount === 1) {
        Dialog.alert({
          title: '폰트 업로드 완료',
          message: `"${results[0].name}" 폰트가 추가되었어요. 업로드한 폰트는 설정 > 폰트 탭에서 관리할 수 있어요.`,
        });
      } else {
        const fontNames = results
          .filter((r) => r.success)
          .map((r) => r.name)
          .join(', ');
        Dialog.alert({
          title: '폰트 업로드 완료',
          message: `${successCount}개의 폰트(${fontNames})가 추가되었어요. 업로드한 폰트는 설정 > 폰트 탭에서 관리할 수 있어요.`,
        });
      }
      open = false;
    } else if (successCount === 0) {
      const errorMessages = results.map((r) => `• ${r.name}: ${r.error}`).join('\n');
      Dialog.alert({
        title: '폰트 업로드 실패',
        message: `모든 폰트 업로드에 실패했어요.\n\n${errorMessages}`,
      });
    } else {
      const successNames = results
        .filter((r) => r.success)
        .map((r) => `"${r.name}"`)
        .join(', ');
      const failureMessages = results
        .filter((r) => !r.success)
        .map((r) => `• ${r.name}: ${r.error}`)
        .join('\n');
      Dialog.alert({
        title: '폰트 업로드 일부 완료',
        message: `${successCount}개의 폰트(${successNames})가 추가되었어요.\n\n다음 ${failureCount}개의 폰트는 업로드에 실패했어요:\n${failureMessages}\n\n업로드된 폰트는 설정 > 폰트 탭에서 관리할 수 있어요.`,
      });
    }
  };

  const handleUpload = async () => {
    const picker = document.createElement('input');
    picker.type = 'file';
    picker.accept = '.ttf,.otf';
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

    const fontFiles = [...files].filter((file) => /\.(ttf|otf)$/i.test(file.name));

    if (fontFiles.length === 0) {
      Dialog.alert({
        title: '올바른 폰트 파일이 아니에요',
        message: 'TTF 또는 OTF 파일만 업로드할 수 있어요.',
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
        <li>TTF, OTF 확장자를 가진 폰트 파일을 업로드할 수 있어요.</li>
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
            <div class={css({ fontSize: '12px', color: 'text.subtle' })}>TTF, OTF 파일 (여러 개 가능)</div>
          </div>
        {/if}
      </div>
    </div>
  </div>
</Modal>
