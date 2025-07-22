<script lang="ts">
  import GlobeIcon from '~icons/lucide/globe';
  import MoonIcon from '~icons/lucide/moon';
  import Share2Icon from '~icons/lucide/share-2';
  import SmartphoneIcon from '~icons/lucide/smartphone';
  import SunIcon from '~icons/lucide/sun';
  import UsersIcon from '~icons/lucide/users';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import Bento from './Bento.svelte';
</script>

<section
  class={css({
    position: 'relative',
    paddingX: '40px',
    paddingY: '120px',
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
      <Icon icon={GlobeIcon} size={16} />
      연결과 확장
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
      함께,
      <span
        class={css({
          backgroundColor: 'amber.400',
          paddingX: '20px',
          display: 'inline-block',
          transform: 'rotate(1deg)',
        })}
      >
        언제 어디서나
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
      혼자가 아닌 함께, 한 곳이 아닌 어디서나.
      <br />
      글쓰기의 경계를 넘어 더 넓은 가능성을 만나보세요.
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
        <div class={center({ flexGrow: 1 })}>
          <div class={css({ position: 'relative', width: 'full', maxWidth: '360px' })}>
            <div
              class={css({
                padding: '24px',
                backgroundColor: 'white',
                border: '4px solid',
                borderColor: 'gray.900',
                boxShadow: '[6px 6px 0 0 #000]',
                position: 'relative',
              })}
            >
              <div class={css({ fontSize: '15px', lineHeight: '[1.8]', color: 'gray.800', fontWeight: 'medium' })}>
                <p class={css({ marginBottom: '16px' })}>
                  그날 밤, 달빛이 창문으로 스며들었다.
                  <span
                    class={css({
                      position: 'relative',
                      backgroundColor: 'blue.100',
                      paddingX: '2px',
                    })}
                  >
                    고요한 침묵 속에서 시계 초침 소리만이
                    <span
                      class={css({
                        position: 'relative',
                        borderXWidth: '3px',
                        borderColor: 'blue.600',
                        _before: {
                          content: '"쿼카"',
                          position: 'absolute',
                          top: '0',
                          left: '-3px',
                          paddingX: '8px',
                          paddingY: '4px',
                          width: 'max',
                          fontSize: '13px',
                          fontWeight: 'black',
                          lineHeight: 'none',
                          color: 'white',
                          backgroundColor: 'blue.600',
                          textTransform: 'uppercase',
                          translateY: '-full',
                          border: '3px solid',
                          borderColor: 'gray.900',
                          boxShadow: '[3px 3px 0 0 #000]',
                        },
                      })}
                    ></span>
                  </span>
                  규칙적으로 울려 퍼졌고, 나는 오랫동안 잊고 있던 기억의 파편들을 하나씩 떠올렸다.
                </p>
                <p>
                  <span
                    class={css({
                      position: 'relative',
                      backgroundColor: 'amber.100',
                      paddingX: '2px',
                    })}
                  >
                    어린 시절 할머니가 들려주던 옛이야기처럼
                    <span
                      class={css({
                        position: 'relative',
                        borderXWidth: '3px',
                        borderColor: 'amber.600',
                        _before: {
                          content: '"카피바라"',
                          position: 'absolute',
                          top: '0',
                          left: '-3px',
                          paddingX: '8px',
                          paddingY: '4px',
                          width: 'max',
                          fontSize: '13px',
                          fontWeight: 'black',
                          lineHeight: 'none',
                          color: 'white',
                          backgroundColor: 'amber.600',
                          textTransform: 'uppercase',
                          translateY: '-full',
                          border: '3px solid',
                          borderColor: 'gray.900',
                          boxShadow: '[3px 3px 0 0 #000]',
                        },
                      })}
                    ></span>
                  </span>
                  그 순간은 영원처럼 느껴졌다. 마치 시간이 멈춘 듯, 모든 것이 선명하게 다가왔다.
                </p>
              </div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={UsersIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>동시편집</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
            여러 사람이 동시에 하나의 문서를 편집할 수 있습니다. 실시간으로 변경사항이 반영되어 효율적인 협업이 가능합니다.
          </p>
        </div>
      </Bento>
    </div>
  </div>

  <div
    class={flex({
      gap: '24px',
    })}
  >
    <div class={css({ flex: '1' })}>
      <Bento>
        <div class={center({ flexGrow: 1 })}>
          <div class={css({ width: 'full', maxWidth: '280px' })}>
            <div
              class={css({
                backgroundColor: 'white',
                border: '4px solid',
                borderColor: 'gray.900',
                padding: '20px',
                marginBottom: '20px',
                boxShadow: '[6px 6px 0 0 #000]',
                transform: 'rotate(-1deg)',
              })}
            >
              <div class={css({ marginBottom: '16px' })}>
                <div class={css({ height: '12px', backgroundColor: 'gray.900', marginBottom: '12px' })}></div>
                <div class={css({ height: '8px', backgroundColor: 'gray.300', marginBottom: '8px' })}></div>
                <div class={css({ height: '8px', backgroundColor: 'gray.300', marginBottom: '8px' })}></div>
                <div class={css({ height: '8px', backgroundColor: 'gray.300', width: '[70%]' })}></div>
              </div>

              <div
                class={css({
                  display: 'flex',
                  alignItems: 'center',
                  gap: '8px',
                  padding: '12px',
                  backgroundColor: 'gray.100',
                  border: '3px solid',
                  borderColor: 'gray.900',
                })}
              >
                <div
                  class={css({ flex: '1', fontSize: '13px', fontFamily: 'mono', color: 'gray.700', truncate: true, fontWeight: 'bold' })}
                >
                  typie.me/123456789
                </div>
                <button
                  class={css({
                    paddingX: '14px',
                    paddingY: '8px',
                    backgroundColor: 'gray.900',
                    color: 'white',
                    fontSize: '12px',
                    fontWeight: 'black',
                    cursor: 'pointer',
                    whiteSpace: 'nowrap',
                    textTransform: 'uppercase',
                    border: '3px solid',
                    borderColor: 'gray.900',
                    boxShadow: '[3px 3px 0 0 #fbbf24]',
                    transition: 'transform',
                    transitionDuration: '150ms',
                    _hover: {
                      transform: 'translate(-2px, -2px)',
                      boxShadow: '[5px 5px 0 0 #fbbf24]',
                    },
                  })}
                  type="button"
                >
                  복사
                </button>
              </div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={Share2Icon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>공유</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
            링크 하나로 쉽게 글을 공유하세요. 읽기 전용, 편집 가능 등 권한을 설정하고 비밀번호로 보호할 수 있습니다.
          </p>
        </div>
      </Bento>
    </div>

    <div class={css({ flex: '1' })}>
      <Bento>
        <div class={center({ flexGrow: 1 })}>
          <div
            class={css({
              position: 'relative',
              width: '240px',
              height: '180px',
              overflow: 'hidden',
              border: '4px solid',
              borderColor: 'gray.900',
              boxShadow: '[6px 6px 0 0 #000]',
            })}
          >
            <div
              class={css({
                position: 'absolute',
                top: '0',
                left: '0',
                width: '[50%]',
                height: 'full',
                backgroundColor: 'gray.900',
              })}
            ></div>

            <div
              class={css({
                position: 'absolute',
                top: '0',
                right: '0',
                width: '[50%]',
                height: 'full',
                backgroundColor: 'white',
                borderLeft: '4px solid',
                borderColor: 'gray.900',
              })}
            ></div>

            <div
              class={css({
                position: 'absolute',
                top: '20px',
                left: '0',
                right: '0',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                paddingX: '20px',
              })}
            >
              <div
                class={css({
                  backgroundColor: 'white',
                  padding: '6px',
                  border: '3px solid',
                  borderColor: 'gray.900',
                })}
              >
                <Icon style={css.raw({ color: 'gray.900' })} icon={MoonIcon} size={20} />
              </div>

              <div
                class={css({
                  backgroundColor: 'amber.400',
                  padding: '6px',
                  border: '3px solid',
                  borderColor: 'gray.900',
                })}
              >
                <Icon style={css.raw({ color: 'gray.900' })} icon={SunIcon} size={20} />
              </div>
            </div>

            <div
              class={css({
                position: 'absolute',
                top: '80px',
                left: '0',
                right: '0',
                paddingX: '20px',
              })}
            >
              <p
                class={css({ textAlign: 'center', fontSize: '14px', lineHeight: '[1.6]', fontWeight: 'bold', textTransform: 'uppercase' })}
              >
                어두운 환경에서도
                <br />
                편안하게 읽고 쓸 수
                <br />
                있습니다
              </p>

              <div
                class={css({
                  position: 'absolute',
                  top: '0',
                  left: '0',
                  width: '[50%]',
                  height: 'full',
                  overflow: 'hidden',
                  color: 'white',
                })}
              >
                <p
                  class={css({
                    textAlign: 'center',
                    fontSize: '14px',
                    lineHeight: '[1.6]',
                    width: '[200%]',
                    fontWeight: 'bold',
                    textTransform: 'uppercase',
                  })}
                >
                  어두운 환경에서도
                  <br />
                  편안하게 읽고 쓸 수
                  <br />
                  있습니다
                </p>
              </div>

              <div
                class={css({
                  position: 'absolute',
                  top: '0',
                  right: '0',
                  width: '[50%]',
                  height: 'full',
                  overflow: 'hidden',
                  color: 'gray.900',
                })}
              >
                <p
                  class={css({
                    textAlign: 'center',
                    fontSize: '14px',
                    lineHeight: '[1.6]',
                    width: '[200%]',
                    marginLeft: '[-100%]',
                    fontWeight: 'bold',
                    textTransform: 'uppercase',
                  })}
                >
                  어두운 환경에서도
                  <br />
                  편안하게 읽고 쓸 수
                  <br />
                  있습니다
                </p>
              </div>
            </div>

            <div
              class={css({
                position: 'absolute',
                bottom: '20px',
                left: '20px',
                right: '20px',
              })}
            >
              <div
                class={css({
                  position: 'absolute',
                  left: '0',
                  width: '[calc(50% - 10px)]',
                })}
              >
                <div class={css({ height: '5px', backgroundColor: 'gray.700', marginBottom: '6px' })}></div>
                <div class={css({ height: '5px', backgroundColor: 'gray.700', marginBottom: '6px' })}></div>
                <div class={css({ height: '5px', backgroundColor: 'gray.700', width: '[70%]' })}></div>
              </div>

              <div
                class={css({
                  position: 'absolute',
                  right: '0',
                  width: '[calc(50% - 10px)]',
                })}
              >
                <div class={css({ height: '5px', backgroundColor: 'gray.300', marginBottom: '6px' })}></div>
                <div class={css({ height: '5px', backgroundColor: 'gray.300', marginBottom: '6px' })}></div>
                <div class={css({ height: '5px', backgroundColor: 'gray.300', width: '[70%]', marginLeft: 'auto' })}></div>
              </div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={MoonIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>다크모드</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
            눈의 피로를 줄이는 다크모드를 지원합니다. 시간대에 따라 자동으로 전환되도록 설정할 수도 있습니다.
          </p>
        </div>
      </Bento>
    </div>

    <div class={css({ flex: '1' })}>
      <Bento>
        <div class={center({ flexGrow: 1 })}>
          <div class={flex({ gap: '16px', alignItems: 'center' })}>
            <div
              class={css({
                width: '100px',
                height: '200px',
                backgroundColor: 'gray.900',
                border: '4px solid',
                borderColor: 'gray.900',
                position: 'relative',
                overflow: 'hidden',
                boxShadow: '[6px 6px 0 0 #000]',
              })}
            >
              <div
                class={css({
                  position: 'absolute',
                  top: '8px',
                  left: '[50%]',
                  transform: 'translateX(-50%)',
                  width: '50px',
                  height: '6px',
                  backgroundColor: 'gray.800',
                })}
              ></div>

              <div
                class={css({
                  marginTop: '25px',
                  padding: '12px',
                  backgroundColor: 'white',
                  margin: '3px',
                  height: '[calc(100% - 40px)]',
                })}
              >
                <div class={css({ height: '10px', backgroundColor: 'gray.900', marginBottom: '10px' })}></div>

                <div class={css({ height: '5px', backgroundColor: 'gray.300', marginBottom: '6px' })}></div>
                <div class={css({ height: '5px', backgroundColor: 'gray.300', marginBottom: '6px' })}></div>
                <div class={css({ height: '5px', backgroundColor: 'gray.300', width: '[75%]', marginBottom: '6px' })}></div>

                <div class={css({ marginTop: '16px' })}>
                  <div class={css({ height: '5px', backgroundColor: 'gray.300', marginBottom: '6px' })}></div>
                  <div class={css({ height: '5px', backgroundColor: 'gray.300', width: '[60%]' })}></div>
                </div>
              </div>

              <div
                class={css({
                  position: 'absolute',
                  bottom: '8px',
                  left: '[50%]',
                  transform: 'translateX(-50%)',
                  width: '30px',
                  height: '5px',
                  backgroundColor: 'white',
                })}
              ></div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={SmartphoneIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>모바일 앱</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
            iOS와 Android 앱으로 언제 어디서나 글을 쓸 수 있습니다. 모든 기기에서 자동으로 동기화됩니다.
          </p>
        </div>
      </Bento>
    </div>
  </div>
</section>
