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
        gap: '3px',
        width: 'full',
        justifyContent: 'stretch',
        padding: '40px',
        backgroundColor: 'white',
        border: '4px solid',
        borderColor: 'gray.900',
        boxShadow: '[8px 8px 0 0 #000]',
      })}
    >
      {#each Array.from({ length: 364 }, (_, i) => i) as cellIndex (cellIndex)}
        <div
          style:grid-row={(cellIndex % 7) + 1}
          style:grid-column={Math.floor(cellIndex / 7) + 1}
          class={css(
            {
              aspectRatio: '1/1',
              position: 'relative',
              border: '2px solid',
              borderColor: 'gray.900',
              transition: 'transform',
              transitionDuration: '100ms',
              cursor: 'pointer',
              _hover: {
                transform: 'scale(1.2)',
                zIndex: '10',
              },
            },
            (() => {
              const seed = cellIndex * 2_654_435_761;
              const random = ((seed ^ (seed >>> 16)) * 0x4_5d_9f_3b) >>> 0;
              const normalized = (random % 100) / 100;

              if (normalized < 0.05) return { backgroundColor: 'gray.200' };
              if (normalized < 0.3) return { backgroundColor: 'amber.200' };
              if (normalized < 0.6) return { backgroundColor: 'amber.300' };
              if (normalized < 0.8) return { backgroundColor: 'amber.400' };
              if (normalized < 0.95) return { backgroundColor: 'amber.500' };
              return { backgroundColor: 'amber.600' };
            })(),
          )}
        >
          {#if cellIndex === 256}
            <div
              class={flex({
                position: 'absolute',
                flexDirection: 'column',
                paddingX: '12px',
                paddingY: '8px',
                color: 'gray.900',
                backgroundColor: 'amber.400',
                zIndex: '20',
                fontSize: '12px',
                fontWeight: 'bold',
                whiteSpace: 'nowrap',
                bottom: '[calc(50% + 32px)]',
                left: '[50%]',
                transform: 'translateX(-50%) rotate(-2deg)',
                border: '4px solid',
                borderColor: 'gray.900',
                boxShadow: '[4px 4px 0 0 #000]',
                textTransform: 'uppercase',
              })}
            >
              <div class={css({ fontWeight: 'black' })}>10월 10일</div>
              <div class={css({ fontWeight: 'black' })}>3,241자 작성</div>
              <div
                class={css({
                  position: 'absolute',
                  bottom: '-8px',
                  left: '[50%]',
                  transform: 'translateX(-50%)',
                  width: '0',
                  height: '0',
                  borderLeft: '[8px solid transparent]',
                  borderRight: '[8px solid transparent]',
                  borderTop: '[8px solid]',
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
          paddingX: '20px',
          paddingY: '8px',
          marginBottom: '32px',
          backgroundColor: 'gray.900',
          color: 'white',
          fontSize: '14px',
          fontWeight: 'bold',
          letterSpacing: '[0.1em]',
          textTransform: 'uppercase',
          transform: 'rotate(-2deg)',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[4px 4px 0 0 #fbbf24]',
        })}
      >
        <Icon icon={TrendingUpIcon} size={16} />
        데이터로 보는 글쓰기
      </div>

      <h2
        class={css({
          fontSize: '[56px]',
          fontWeight: 'black',
          color: 'gray.950',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          marginBottom: '24px',
          lineHeight: '[1.1]',
          textTransform: 'uppercase',
        })}
      >
        데이터로 보는
        <br />
        <span
          class={css({
            backgroundColor: 'amber.400',
            paddingX: '20px',
            display: 'inline-block',
            transform: 'rotate(1deg)',
          })}
        >
          작성 기록
        </span>
      </h2>
      <p
        class={css({
          fontSize: '20px',
          fontWeight: 'semibold',
          color: 'gray.700',
          textAlign: 'center',
          fontFamily: 'Pretendard',
          maxWidth: '600px',
          lineHeight: '[1.7]',
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
              <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>
                목표를 향한 실시간 가이드
              </h3>
            </div>
            <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
              실시간으로 업데이트되는 글자 수 통계로 마감이나 분량 목표를 쉽게 관리할 수 있습니다.
            </p>
          </div>

          <div class={center({ flexGrow: 1 })}>
            <div class={css({ width: 'full', maxWidth: '380px' })}>
              <div
                class={css({
                  paddingX: '20px',
                  paddingY: '16px',
                  backgroundColor: 'white',
                  border: '4px solid',
                  borderColor: 'gray.900',
                  boxShadow: '[6px 6px 0 0 #fbbf24]',
                  transform: 'rotate(-1deg)',
                  transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                  _hover: {
                    transform: 'rotate(0deg) translate(-2px, -2px)',
                    boxShadow: '[8px 8px 0 0 #fbbf24]',
                  },
                })}
              >
                <div
                  class={css({
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    marginBottom: '16px',
                    cursor: 'pointer',
                  })}
                >
                  <Icon style={css.raw({ color: 'gray.900' })} icon={TypeIcon} size={14} />
                  <span class={css({ fontSize: '14px', fontWeight: 'black', color: 'gray.900', textTransform: 'uppercase' })}>글자 수</span>
                  <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(90deg)' })} icon={ChevronRightIcon} size={14} />
                </div>

                <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                  <div class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '14px' })}>
                    <span class={css({ color: 'gray.700', fontWeight: 'semibold' })}>공백 포함</span>
                    <span class={css({ fontWeight: 'bold', color: 'gray.900' })}>
                      <span class={css({ color: 'amber.600', fontWeight: 'black' })}>125자</span>
                      / 523자
                    </span>
                  </div>

                  <div class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '14px' })}>
                    <span class={css({ color: 'gray.700', fontWeight: 'semibold' })}>공백 미포함</span>
                    <span class={css({ fontWeight: 'bold', color: 'gray.900' })}>
                      <span class={css({ color: 'amber.600', fontWeight: 'black' })}>98자</span>
                      / 456자
                    </span>
                  </div>

                  <div class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '14px' })}>
                    <span class={css({ color: 'gray.700', fontWeight: 'semibold' })}>공백/부호 미포함</span>
                    <span class={css({ fontWeight: 'bold', color: 'gray.900' })}>
                      <span class={css({ color: 'amber.600', fontWeight: 'black' })}>87자</span>
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
              <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>
                오늘의 작성량 확인
              </h3>
            </div>
            <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
              오늘 추가하고 삭제한 글자 수를 추적하여, 순수한 작성량을 정확히 파악할 수 있습니다.
            </p>
          </div>

          <div class={center({ flexGrow: 1 })}>
            <div class={css({ width: 'full', maxWidth: '380px' })}>
              <div
                class={css({
                  paddingX: '20px',
                  paddingY: '16px',
                  backgroundColor: 'white',
                  border: '4px solid',
                  borderColor: 'gray.900',
                  boxShadow: '[6px 6px 0 0 #10b981]',
                  transform: 'rotate(1deg)',
                  transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                  _hover: {
                    transform: 'rotate(0deg) translate(-2px, -2px)',
                    boxShadow: '[8px 8px 0 0 #10b981]',
                  },
                })}
              >
                <div
                  class={css({
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    marginBottom: '16px',
                    cursor: 'pointer',
                  })}
                >
                  <Icon style={css.raw({ color: 'gray.900' })} icon={GoalIcon} size={14} />
                  <span class={css({ fontSize: '14px', fontWeight: 'black', color: 'gray.900', textTransform: 'uppercase' })}>
                    오늘의 기록
                  </span>
                  <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(90deg)' })} icon={ChevronRightIcon} size={14} />
                </div>

                <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                  <div class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '14px' })}>
                    <span class={css({ color: 'gray.700', fontWeight: 'semibold' })}>변화량</span>
                    <div class={flex({ alignItems: 'center', gap: '4px', fontWeight: 'bold', color: 'gray.900' })}>
                      <Icon style={css.raw({ color: 'green.600' })} icon={TrendingUpIcon} size={14} />
                      <span class={css({ color: 'green.700', fontWeight: 'black' })}>+156자</span>
                    </div>
                  </div>

                  <div class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '14px' })}>
                    <span class={css({ color: 'gray.700', fontWeight: 'semibold' })}>입력한 글자</span>
                    <span class={css({ fontWeight: 'bold', color: 'gray.900' })}>523자</span>
                  </div>

                  <div class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '14px' })}>
                    <span class={css({ color: 'gray.700', fontWeight: 'semibold' })}>지운 글자</span>
                    <span class={css({ fontWeight: 'bold', color: 'gray.900' })}>367자</span>
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
