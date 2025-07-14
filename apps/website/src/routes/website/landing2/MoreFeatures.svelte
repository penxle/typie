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
      <Icon icon={GlobeIcon} size={16} />
      연결과 확장
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
      함께, 언제 어디서나
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
            <!-- 에디터 목업 -->
            <div
              class={css({
                padding: '24px',
                borderRadius: '12px',
                backgroundColor: 'white',
                border: '[1px solid]',
                borderColor: 'gray.200',
                boxShadow: 'small',
                position: 'relative',
              })}
            >
              <!-- 텍스트 컨텐츠 -->
              <div class={css({ fontSize: '15px', lineHeight: '[1.8]', color: 'gray.800' })}>
                <p class={css({ marginBottom: '16px' })}>
                  그날 밤, 달빛이 창문으로 스며들었다.
                  <span
                    class={css({
                      position: 'relative',
                      backgroundColor: '[color-mix(in srgb, #3b82f6 20%, transparent)]',
                    })}
                  >
                    고요한 침묵 속에서 시계 초침 소리만이
                    <!-- 쿼카 커서 -->
                    <span
                      class={css({
                        position: 'relative',
                        marginX: '-1px',
                        borderXWidth: '1px',
                        borderColor: '[#3b82f6]',
                        _before: {
                          content: '"쿼카"',
                          position: 'absolute',
                          top: '0',
                          left: '-1px',
                          borderTopLeftRadius: '4px',
                          borderTopRightRadius: '4px',
                          borderBottomRightRadius: '4px',
                          paddingX: '6px',
                          paddingY: '4px',
                          width: 'max',
                          fontSize: '13px',
                          fontWeight: 'medium',
                          lineHeight: 'none',
                          color: 'white',
                          backgroundColor: '[#3b82f6]',
                          translate: 'auto',
                          translateY: '-full',
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
                      backgroundColor: '[color-mix(in srgb, #f59e0b 20%, transparent)]',
                    })}
                  >
                    어린 시절 할머니가 들려주던 옛이야기처럼
                    <!-- 카피바라 커서 -->
                    <span
                      class={css({
                        position: 'relative',
                        marginX: '-1px',
                        borderXWidth: '1px',
                        borderColor: '[#f59e0b]',
                        _before: {
                          content: '"카피바라"',
                          position: 'absolute',
                          top: '0',
                          left: '-1px',
                          borderTopLeftRadius: '4px',
                          borderTopRightRadius: '4px',
                          borderBottomRightRadius: '4px',
                          paddingX: '6px',
                          paddingY: '4px',
                          width: 'max',
                          fontSize: '13px',
                          fontWeight: 'medium',
                          lineHeight: 'none',
                          color: 'white',
                          backgroundColor: '[#f59e0b]',
                          translate: 'auto',
                          translateY: '-full',
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
            <h3 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'gray.800' })}>동시편집</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]' })}>
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
            <!-- 문서 미리보기 -->
            <div
              class={css({
                backgroundColor: 'white',
                borderRadius: '12px',
                border: '[1px solid]',
                borderColor: 'gray.200',
                padding: '20px',
                marginBottom: '20px',
                boxShadow: 'small',
              })}
            >
              <div class={css({ marginBottom: '16px' })}>
                <div class={css({ height: '10px', backgroundColor: 'gray.800', borderRadius: '5px', marginBottom: '12px' })}></div>
                <div class={css({ height: '8px', backgroundColor: 'gray.200', borderRadius: '4px', marginBottom: '8px' })}></div>
                <div class={css({ height: '8px', backgroundColor: 'gray.200', borderRadius: '4px', marginBottom: '8px' })}></div>
                <div class={css({ height: '8px', backgroundColor: 'gray.200', borderRadius: '4px', width: '[70%]' })}></div>
              </div>

              <!-- 공유 URL -->
              <div
                class={css({
                  display: 'flex',
                  alignItems: 'center',
                  gap: '8px',
                  padding: '12px',
                  backgroundColor: 'gray.50',
                  borderRadius: '8px',
                })}
              >
                <div class={css({ flex: '1', fontSize: '13px', fontFamily: 'mono', color: 'gray.600', truncate: true })}>
                  typie.me/123456789
                </div>
                <button
                  class={css({
                    paddingX: '12px',
                    paddingY: '6px',
                    borderRadius: '6px',
                    backgroundColor: 'gray.900',
                    color: 'white',
                    fontSize: '12px',
                    fontWeight: 'medium',
                    cursor: 'pointer',
                    whiteSpace: 'nowrap',
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
            <h3 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'gray.800' })}>공유</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]' })}>
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
              borderRadius: '12px',
              overflow: 'hidden',
              boxShadow: 'medium',
            })}
          >
            <!-- 전체 카드 배경 (두 개의 절반) -->
            <!-- 왼쪽 절반 - 다크모드 -->
            <div
              class={css({
                position: 'absolute',
                top: '0',
                left: '0',
                width: '[50%]',
                height: 'full',
                backgroundColor: 'dark.gray.900',
              })}
            ></div>

            <!-- 오른쪽 절반 - 라이트모드 -->
            <div
              class={css({
                position: 'absolute',
                top: '0',
                right: '0',
                width: '[50%]',
                height: 'full',
                backgroundColor: 'white',
                borderLeft: '[1px solid]',
                borderColor: 'gray.200',
              })}
            ></div>

            <!-- 상단 아이콘/토글 영역 -->
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
              <!-- 다크모드 아이콘 (왼쪽) -->
              <Icon style={css.raw({ color: 'white' })} icon={MoonIcon} size={20} />

              <!-- 라이트모드 아이콘 (오른쪽) -->
              <Icon style={css.raw({ color: 'gray.800' })} icon={SunIcon} size={20} />
            </div>

            <!-- 텍스트 콘텐츠 (전체 가운데 정렬로 텍스트가 이어지는 효과) -->
            <div
              class={css({
                position: 'absolute',
                top: '70px',
                left: '0',
                right: '0',
                paddingX: '20px',
              })}
            >
              <!-- 전체 텍스트 (가운데 정렬) -->
              <p class={css({ textAlign: 'center', fontSize: '14px', lineHeight: '[1.6]' })}>
                어두운 환경에서도
                <br />
                편안하게 읽고 쓸 수
                <br />
                있습니다
              </p>

              <!-- 다크모드 영역만 보이게 하는 마스크 -->
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
                <p class={css({ textAlign: 'center', fontSize: '14px', lineHeight: '[1.6]', width: '[200%]' })}>
                  어두운 환경에서도
                  <br />
                  편안하게 읽고 쓸 수
                  <br />
                  있습니다
                </p>
              </div>

              <!-- 라이트모드 영역만 보이게 하는 마스크 -->
              <div
                class={css({
                  position: 'absolute',
                  top: '0',
                  right: '0',
                  width: '[50%]',
                  height: 'full',
                  overflow: 'hidden',
                  color: 'gray.800',
                })}
              >
                <p class={css({ textAlign: 'center', fontSize: '14px', lineHeight: '[1.6]', width: '[200%]', marginLeft: '[-100%]' })}>
                  어두운 환경에서도
                  <br />
                  편안하게 읽고 쓸 수
                  <br />
                  있습니다
                </p>
              </div>
            </div>

            <!-- 미리보기 라인들을 위한 컨테이너 -->
            <div
              class={css({
                position: 'absolute',
                bottom: '20px',
                left: '20px',
                right: '20px',
              })}
            >
              <!-- 왼쪽 다크모드 라인들 -->
              <div
                class={css({
                  position: 'absolute',
                  left: '0',
                  width: '[calc(50% - 10px)]',
                })}
              >
                <div class={css({ height: '4px', backgroundColor: 'dark.gray.700', borderRadius: '2px', marginBottom: '6px' })}></div>
                <div class={css({ height: '4px', backgroundColor: 'dark.gray.700', borderRadius: '2px', marginBottom: '6px' })}></div>
                <div class={css({ height: '4px', backgroundColor: 'dark.gray.700', borderRadius: '2px', width: '[70%]' })}></div>
              </div>

              <!-- 오른쪽 라이트모드 라인들 -->
              <div
                class={css({
                  position: 'absolute',
                  right: '0',
                  width: '[calc(50% - 10px)]',
                })}
              >
                <div class={css({ height: '4px', backgroundColor: 'gray.200', borderRadius: '2px', marginBottom: '6px' })}></div>
                <div class={css({ height: '4px', backgroundColor: 'gray.200', borderRadius: '2px', marginBottom: '6px' })}></div>
                <div
                  class={css({ height: '4px', backgroundColor: 'gray.200', borderRadius: '2px', width: '[70%]', marginLeft: 'auto' })}
                ></div>
              </div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={MoonIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'gray.800' })}>다크모드</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]' })}>
            눈의 피로를 줄이는 다크모드를 지원합니다. 시간대에 따라 자동으로 전환되도록 설정할 수도 있습니다.
          </p>
        </div>
      </Bento>
    </div>

    <div class={css({ flex: '1' })}>
      <Bento>
        <div class={center({ flexGrow: 1 })}>
          <div class={flex({ gap: '16px', alignItems: 'center' })}>
            <!-- 모바일 기기 목업 -->
            <div
              class={css({
                width: '100px',
                height: '200px',
                borderRadius: '16px',
                backgroundColor: 'gray.900',
                border: '[3px solid]',
                borderColor: 'gray.800',
                position: 'relative',
                overflow: 'hidden',
                boxShadow: 'medium',
              })}
            >
              <!-- 노치 -->
              <div
                class={css({
                  position: 'absolute',
                  top: '8px',
                  left: '[50%]',
                  transform: 'translateX(-50%)',
                  width: '50px',
                  height: '5px',
                  borderRadius: 'full',
                  backgroundColor: 'gray.800',
                })}
              ></div>

              <!-- 화면 콘텐츠 -->
              <div
                class={css({
                  marginTop: '25px',
                  padding: '12px',
                })}
              >
                <!-- 타이틀 -->
                <div class={css({ height: '8px', backgroundColor: 'gray.700', borderRadius: '4px', marginBottom: '10px' })}></div>

                <!-- 텍스트 라인들 -->
                <div class={css({ height: '4px', backgroundColor: 'gray.800', borderRadius: '2px', marginBottom: '6px' })}></div>
                <div class={css({ height: '4px', backgroundColor: 'gray.800', borderRadius: '2px', marginBottom: '6px' })}></div>
                <div
                  class={css({ height: '4px', backgroundColor: 'gray.800', borderRadius: '2px', width: '[75%]', marginBottom: '6px' })}
                ></div>

                <!-- 추가 콘텐츠 -->
                <div class={css({ marginTop: '16px' })}>
                  <div class={css({ height: '4px', backgroundColor: 'gray.800', borderRadius: '2px', marginBottom: '6px' })}></div>
                  <div class={css({ height: '4px', backgroundColor: 'gray.800', borderRadius: '2px', width: '[60%]' })}></div>
                </div>
              </div>

              <!-- 하단 홈 인디케이터 -->
              <div
                class={css({
                  position: 'absolute',
                  bottom: '8px',
                  left: '[50%]',
                  transform: 'translateX(-50%)',
                  width: '30px',
                  height: '4px',
                  borderRadius: 'full',
                  backgroundColor: 'gray.700',
                })}
              ></div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={SmartphoneIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'gray.800' })}>모바일 앱</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]' })}>
            iOS와 Android 앱으로 언제 어디서나 글을 쓸 수 있습니다. 모든 기기에서 자동으로 동기화됩니다.
          </p>
        </div>
      </Bento>
    </div>
  </div>
</section>
