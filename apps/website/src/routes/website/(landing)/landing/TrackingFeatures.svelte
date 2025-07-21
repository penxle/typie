<script lang="ts">
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import GoalIcon from '~icons/lucide/goal';
  import TrendingUpIcon from '~icons/lucide/trending-up';
  import TypeIcon from '~icons/lucide/type';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex, grid } from '$styled-system/patterns';
  import Bento from './Bento.svelte';
</script>

<section
  class={css({
    position: 'relative',
  })}
>
  <div class={css({ width: 'full', position: 'relative', marginBottom: '120px' })}>
    <div
      class={grid({
        gridTemplateRows: 'repeat(7, minmax(0, 1fr))',
        gridTemplateColumns: 'repeat(52, minmax(0, 1fr))',
        gridAutoFlow: 'column',
        gap: '2px',
        width: 'full',
        justifyContent: 'stretch',
      })}
    >
      {#each Array.from({ length: 364 }, (_, i) => i) as cellIndex (cellIndex)}
        <div
          style:grid-row={(cellIndex % 7) + 1}
          style:grid-column={Math.floor(cellIndex / 7) + 1}
          class={css(
            { borderRadius: '2px', aspectRatio: '1/1', position: 'relative' },
            (() => {
              const seed = cellIndex * 2_654_435_761;
              const random = ((seed ^ (seed >>> 16)) * 0x4_5d_9f_3b) >>> 0;
              const normalized = (random % 100) / 100;

              if (normalized < 0.05) return { backgroundColor: 'gray.100' };
              if (normalized < 0.3) return { backgroundColor: 'amber.100' };
              if (normalized < 0.6) return { backgroundColor: 'amber.200' };
              if (normalized < 0.8) return { backgroundColor: 'amber.300' };
              if (normalized < 0.95) return { backgroundColor: 'amber.400' };
              return { backgroundColor: 'amber.500' };
            })(),
          )}
        >
          {#if cellIndex === 256}
            <div
              class={flex({
                position: 'absolute',
                flexDirection: 'column',
                borderRadius: '6px',
                paddingX: '10px',
                paddingY: '6px',
                color: 'white',
                backgroundColor: 'gray.900',
                zIndex: '10',
                fontSize: '12px',
                boxShadow: 'medium',
                whiteSpace: 'nowrap',
                bottom: '[calc(50% + 24px)]',
                left: '[50%]',
                transform: 'translateX(-50%)',
              })}
            >
              <div class={css({ fontWeight: 'medium' })}>10월 10일</div>
              <div class={css({ fontWeight: 'bold' })}>3,241자 작성했어요</div>
              <div
                class={css({
                  position: 'absolute',
                  bottom: '-4px',
                  left: '[50%]',
                  transform: 'translateX(-50%)',
                  width: '0',
                  height: '0',
                  borderLeft: '[4px solid transparent]',
                  borderRight: '[4px solid transparent]',
                  borderTop: '[4px solid]',
                  borderTopColor: 'gray.900',
                })}
              ></div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </div>

  <div
    class={css({
      paddingX: '40px',
      maxWidth: '[1024px]',
      marginX: 'auto',
    })}
  >
    <div class={center({ flexDirection: 'column', marginBottom: '80px' })}>
      <div
        class={css({
          display: 'inline-flex',
          alignItems: 'center',
          gap: '8px',
          paddingX: '16px',
          paddingY: '6px',
          marginBottom: '24px',
          borderRadius: 'full',
          backgroundColor: 'gray.900',
          color: 'white',
          fontSize: '14px',
          fontWeight: 'medium',
        })}
      >
        <Icon icon={TrendingUpIcon} size={16} />
        데이터로 보는 글쓰기
      </div>

      <h2
        class={css({
          fontSize: '[44px]',
          fontWeight: 'extrabold',
          color: 'gray.950',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          marginBottom: '16px',
          lineHeight: '[1.2]',
        })}
      >
        데이터로 보는 나의 작성 기록
      </h2>
      <p
        class={css({
          fontSize: '18px',
          fontWeight: 'medium',
          color: 'gray.600',
          textAlign: 'center',
          fontFamily: 'Pretendard',
          maxWidth: '600px',
          lineHeight: '[1.6]',
        })}
      >
        글쓰기 목표를 설정하고 통계로 과정을 추적해 보세요.
        <br />
        객관적인 데이터가 목표 달성을 함께 돕습니다.
      </p>
    </div>

    <div
      class={flex({
        gap: '24px',
        marginBottom: '24px',
      })}
    >
      <div class={css({ flex: '1' })}>
        <Bento>
          <div class={css({ marginBottom: '24px' })}>
            <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
              <Icon icon={TypeIcon} size={20} />
              <h3 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'gray.800' })}>목표를 향한 실시간 가이드</h3>
            </div>
            <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]' })}>
              실시간으로 업데이트되는 글자 수 통계로 마감이나 분량 목표를 쉽게 관리할 수 있습니다.
            </p>
          </div>

          <div class={center({ flexGrow: 1 })}>
            <div class={css({ width: 'full', maxWidth: '380px' })}>
              <div
                class={css({
                  paddingX: '16px',
                  paddingY: '12px',
                  borderRadius: '8px',
                  backgroundColor: 'white',
                  border: '[1px solid]',
                  borderColor: 'amber.300',
                  boxShadow: 'small',
                })}
              >
                <div
                  class={css({
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    marginBottom: '12px',
                    cursor: 'pointer',
                  })}
                >
                  <Icon style={css.raw({ color: 'gray.600' })} icon={TypeIcon} size={12} />
                  <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.800' })}>글자 수</span>
                  <Icon style={css.raw({ color: 'gray.600', transform: 'rotate(90deg)' })} icon={ChevronRightIcon} size={14} />
                </div>

                <div class={flex({ flexDirection: 'column', gap: '6px' })}>
                  <div class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
                    <span class={css({ color: 'gray.600' })}>공백 포함</span>
                    <span class={css({ fontWeight: 'medium', color: 'gray.800' })}>
                      <span class={css({ color: 'amber.600' })}>125자</span>
                      / 523자
                    </span>
                  </div>

                  <div class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
                    <span class={css({ color: 'gray.600' })}>공백 미포함</span>
                    <span class={css({ fontWeight: 'medium', color: 'gray.800' })}>
                      <span class={css({ color: 'amber.600' })}>98자</span>
                      / 456자
                    </span>
                  </div>

                  <div class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
                    <span class={css({ color: 'gray.600' })}>공백/부호 미포함</span>
                    <span class={css({ fontWeight: 'medium', color: 'gray.800' })}>
                      <span class={css({ color: 'amber.600' })}>87자</span>
                      / 389자
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </Bento>
      </div>

      <div class={css({ flex: '1' })}>
        <Bento>
          <div class={css({ marginBottom: '24px' })}>
            <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
              <Icon icon={GoalIcon} size={20} />
              <h3 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'gray.800' })}>오늘의 작성량 확인</h3>
            </div>
            <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]' })}>
              오늘 추가하고 삭제한 글자 수를 추적하여, 순수한 작성량을 정확히 파악할 수 있습니다.
            </p>
          </div>

          <div class={center({ flexGrow: 1 })}>
            <div class={css({ width: 'full', maxWidth: '380px' })}>
              <div
                class={css({
                  paddingX: '16px',
                  paddingY: '12px',
                  borderRadius: '8px',
                  backgroundColor: 'white',
                  border: '[1px solid]',
                  borderColor: 'green.200',
                  boxShadow: 'small',
                })}
              >
                <div
                  class={css({
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    marginBottom: '12px',
                    cursor: 'pointer',
                  })}
                >
                  <Icon style={css.raw({ color: 'gray.600' })} icon={GoalIcon} size={12} />
                  <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.800' })}>오늘의 기록</span>
                  <Icon style={css.raw({ color: 'gray.600', transform: 'rotate(90deg)' })} icon={ChevronRightIcon} size={14} />
                </div>

                <div class={flex({ flexDirection: 'column', gap: '6px' })}>
                  <div class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
                    <span class={css({ color: 'gray.600' })}>변화량</span>
                    <div class={flex({ alignItems: 'center', gap: '4px', fontWeight: 'medium', color: 'gray.800' })}>
                      <Icon style={css.raw({ color: 'green.600' })} icon={TrendingUpIcon} size={14} />
                      <span class={css({ color: 'green.700' })}>+156자</span>
                    </div>
                  </div>

                  <div class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
                    <span class={css({ color: 'gray.600' })}>입력한 글자</span>
                    <span class={css({ fontWeight: 'medium', color: 'gray.800' })}>523자</span>
                  </div>

                  <div class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
                    <span class={css({ color: 'gray.600' })}>지운 글자</span>
                    <span class={css({ fontWeight: 'medium', color: 'gray.800' })}>367자</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </Bento>
      </div>
    </div>
  </div>
</section>
