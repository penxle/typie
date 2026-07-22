<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { enhance } from '$app/forms';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  let agreed = $state(false);
  let submitting = $state(false);
</script>

<main class={css({ minHeight: '[100dvh]', backgroundColor: 'surface.subtle' })}>
  <div class={css({ maxWidth: '640px', marginX: 'auto', paddingY: '64px', paddingX: '20px' })}>
    <header class={css({ marginBottom: '24px' })}>
      <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>평가 참여 전 확인사항</h1>
      <p class={css({ marginTop: '4px', fontSize: '14px', color: 'text.subtle' })}>{data.email} · 처음 한 번만 표시됩니다</p>
    </header>

    <section
      class={css({
        backgroundColor: 'surface.default',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '12px',
        padding: '24px',
        boxShadow: 'small',
      })}
    >
      <div class={flex({ direction: 'column', gap: '20px' })}>
        <div>
          <h2 class={css({ fontSize: '15px', fontWeight: 'bold', marginBottom: '6px' })}>작업 안내</h2>
          <ul class={css({ fontSize: '14px', lineHeight: '[1.7]', color: 'text.default', paddingLeft: '18px', listStyleType: 'disc' })}>
            <li>원문을 충분히 읽은 뒤 피드백 세트를 비교해 순위를 매기는 작업입니다. 한 편에 10–20분쯤 걸립니다.</li>
            <li>세트가 어느 쪽인지 알 수 없도록 가려져 있습니다. 순서나 선입견 없이 내용만으로 판단해 주세요.</li>
            <li>판단이 어려우면 동률을 허용하고, 근거는 코멘트로 남겨주세요. 중간 저장 후 이어서 할 수 있습니다.</li>
          </ul>
        </div>

        <div>
          <h2 class={css({ fontSize: '15px', fontWeight: 'bold', marginBottom: '6px' })}>평가 대상</h2>
          <p class={css({ fontSize: '14px', lineHeight: '[1.7]' })}>
            평가하는 모든 글은 실제 이용자가 작성한 글입니다. 이용자가 직접 공개로 설정한 글 중에서 무작위로 추출되었으며, 이용약관에 따라
            적법하게 제품 개선 목적으로 사용되는 절차입니다.
          </p>
        </div>

        <div>
          <h2 class={css({ fontSize: '15px', fontWeight: 'bold', marginBottom: '6px' })}>기밀 유지</h2>
          <p class={css({ fontSize: '14px', lineHeight: '[1.7]' })}>
            글 내용, 피드백, 순위와 코멘트를 포함한 모든 평가 과정은 외부 유출이 절대 금지됩니다. 화면 캡처, 복사, 제3자 공유 및 어떤 형태의
            재배포도 허용되지 않습니다.
          </p>
        </div>

        <form
          class={css({ borderTopWidth: '1px', borderColor: 'border.subtle', paddingTop: '16px' })}
          method="post"
          use:enhance={() => {
            submitting = true;
            return async ({ update }) => {
              await update();
              submitting = false;
            };
          }}
        >
          <label class={flex({ align: 'center', gap: '8px', fontSize: '14px', cursor: 'pointer' })}>
            <input
              class={css({
                appearance: 'none',
                width: '18px',
                height: '18px',
                borderWidth: '1px',
                borderColor: 'border.strong',
                borderRadius: '4px',
                backgroundColor: 'surface.default',
                cursor: 'pointer',
                transition: '[background-color 0.15s ease, border-color 0.15s ease]',
                _checked: { backgroundColor: 'accent.brand.default', borderColor: 'border.brand' },
              })}
              type="checkbox"
              bind:checked={agreed}
            />
            위 내용을 모두 확인했으며 이에 동의합니다.
          </label>

          <button
            class={css({
              width: 'full',
              marginTop: '16px',
              paddingY: '12px',
              borderRadius: '10px',
              backgroundColor: 'accent.brand.default',
              color: 'text.bright',
              fontSize: '15px',
              fontWeight: 'bold',
              cursor: 'pointer',
              transition: '[background-color 0.15s ease]',
              _disabled: { backgroundColor: 'interactive.disabled', cursor: 'not-allowed' },
              ['&:hover:not(:disabled)']: { backgroundColor: 'accent.brand.hover' },
            })}
            disabled={!agreed || submitting}
            type="submit"
          >
            동의하고 평가 시작하기
          </button>
        </form>
      </div>
    </section>
  </div>
</main>
