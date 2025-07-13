<script lang="ts">
  import BoldIcon from '~icons/lucide/bold';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import CodeIcon from '~icons/lucide/code';
  import CodeXmlIcon from '~icons/lucide/code-xml';
  import CogIcon from '~icons/lucide/cog';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import FileIcon from '~icons/lucide/file';
  import FileUpIcon from '~icons/lucide/file-up';
  import FolderIcon from '~icons/lucide/folder';
  import GalleryVerticalEndIcon from '~icons/lucide/gallery-vertical-end';
  import GoalIcon from '~icons/lucide/goal';
  import ImageIcon from '~icons/lucide/image';
  import ItalicIcon from '~icons/lucide/italic';
  import LinkIcon from '~icons/lucide/link';
  import ListIcon from '~icons/lucide/list';
  import PanelRightCloseIcon from '~icons/lucide/panel-right-close';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import PlusIcon from '~icons/lucide/plus';
  import QuoteIcon from '~icons/lucide/quote';
  import RedoIcon from '~icons/lucide/redo';
  import SearchIcon from '~icons/lucide/search';
  import ShapesIcon from '~icons/lucide/shapes';
  import StrikethroughIcon from '~icons/lucide/strikethrough';
  import TableIcon from '~icons/lucide/table';
  import TrendingUpIcon from '~icons/lucide/trending-up';
  import TypeIcon from '~icons/lucide/type';
  import UnderlineIcon from '~icons/lucide/underline';
  import UndoIcon from '~icons/lucide/undo';
  import HorizontalRuleIcon from '~icons/typie/horizontal-rule';
  import FaviconLight from '$assets/logos/favicon-light.svg?component';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
</script>

<div class={css({ display: 'flex', width: 'full', height: 'full', overflow: 'hidden', backgroundColor: 'gray.100' })}>
  <!-- 왼쪽 사이드바 -->
  <aside
    class={flex({
      flexDirection: 'column',
      alignItems: 'center',
      gap: '24px',
      paddingY: '12px',
      width: '64px',
      flexShrink: '0',
    })}
  >
    <div class={css({ flexShrink: '0', borderRadius: '8px', size: '32px', overflow: 'hidden' })}>
      <FaviconLight class={css({ size: 'full' })} />
    </div>

    <div
      class={center({
        borderWidth: '1px',
        borderColor: 'gray.300',
        borderRadius: '8px',
        size: '32px',
        color: 'gray.500',
        backgroundColor: 'gray.50',
        boxShadow: 'small',
      })}
    >
      <Icon icon={PlusIcon} size={20} />
    </div>

    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div
        class={center({
          borderRadius: '8px',
          size: '32px',
          color: 'gray.700',
          backgroundColor: 'gray.200',
        })}
      >
        <Icon icon={FolderIcon} size={20} />
      </div>

      <div
        class={center({
          borderRadius: '8px',
          size: '32px',
          color: 'gray.500',
        })}
      >
        <Icon icon={SearchIcon} size={20} />
      </div>
    </div>

    <div class={css({ flexGrow: '1' })}></div>

    <div
      class={center({
        borderRadius: '8px',
        size: '32px',
        color: 'gray.500',
      })}
    >
      <Icon icon={CogIcon} size={20} />
    </div>

    <div class={css({ borderRadius: 'full', size: '32px', backgroundColor: 'gray.100' })}></div>
  </aside>

  <!-- 메인 컨테이너 -->
  <div
    class={css({
      flexGrow: '1',
      borderRadius: '4px',
      backgroundColor: 'white',
      boxShadow: '[0 3px 6px -2px {colors.gray.950/3}, 0 1px 1px {colors.gray.950/5}]',
      overflow: 'hidden',
      display: 'flex',
    })}
  >
    <!-- 좌측 네비게이션 패널 -->
    <div class={css({ width: '220px', borderRightWidth: '1px', borderColor: 'gray.100' })}>
      <div
        class={flex({
          flexDirection: 'column',
          width: '220px',
          height: 'full',
          backgroundColor: 'white',
        })}
      >
        <div
          class={flex({
            alignItems: 'center',
            justifyContent: 'space-between',
            paddingX: '16px',
            paddingY: '12px',
            borderBottomWidth: '1px',
            borderColor: 'gray.100',
          })}
        >
          <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'gray.700' })}>내 포스트</div>
          <div
            class={center({
              size: '20px',
              borderRadius: '4px',
              color: 'gray.500',
            })}
          >
            <Icon icon={PlusIcon} size={14} />
          </div>
        </div>

        <div class={flex({ flexDirection: 'column', flexGrow: '1', paddingY: '8px', overflow: 'auto' })}>
          <div
            class={cx(
              'group',
              flex({
                alignItems: 'center',
                gap: '4px',
                paddingX: '8px',
                paddingY: '6px',
                borderRadius: '6px',
              }),
            )}
          >
            <Icon style={css.raw({ color: 'gray.500' })} icon={ChevronDownIcon} size={16} />
            <Icon style={css.raw({ color: 'gray.500' })} icon={FolderIcon} size={14} />
            <span class={css({ flexGrow: '1', fontSize: '14px', fontWeight: 'medium', color: 'gray.600' })}>내 첫 폴더</span>
            <div
              class={center({
                opacity: '0',
                size: '16px',
                borderRadius: '4px',
                color: 'gray.400',
              })}
            >
              <Icon icon={EllipsisIcon} size={14} />
            </div>
          </div>

          <div
            class={cx(
              'group',
              css({
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
                paddingX: '8px',
                paddingY: '6px',
                marginLeft: '16px',
                borderRadius: '6px',
                borderLeftWidth: '1px',
                borderLeftRadius: '0',
                paddingLeft: '14px',
                backgroundColor: 'gray.100',
              }),
            )}
          >
            <div
              class={css({
                flex: 'none',
                borderRadius: 'full',
                backgroundColor: 'gray.200',
                size: '4px',
              })}
            ></div>
            <Icon style={css.raw({ color: 'gray.500' })} icon={FileIcon} size={14} />
            <span
              class={css({
                flexGrow: '1',
                fontSize: '14px',
                fontWeight: 'bold',
                color: 'gray.900',
                wordBreak: 'break-all',
                lineClamp: '1',
              })}
            >
              포스트 제목입니다
            </span>
            <div
              class={center({
                opacity: '0',
                size: '16px',
                borderRadius: '4px',
                color: 'gray.400',
              })}
            >
              <Icon icon={EllipsisIcon} size={14} />
            </div>
          </div>

          <div
            class={cx(
              'group',
              css({
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
                paddingX: '8px',
                paddingY: '6px',
                marginLeft: '16px',
                borderRadius: '6px',
                borderLeftWidth: '1px',
                borderLeftRadius: '0',
                paddingLeft: '14px',
              }),
            )}
          >
            <div
              class={css({
                flex: 'none',
                borderRadius: 'full',
                backgroundColor: 'gray.200',
                size: '4px',
              })}
            ></div>
            <Icon style={css.raw({ color: 'gray.500' })} icon={FileIcon} size={14} />
            <span
              class={css({
                flexGrow: '1',
                fontSize: '14px',
                fontWeight: 'medium',
                color: 'gray.600',
                wordBreak: 'break-all',
                lineClamp: '1',
              })}
            >
              또 다른 포스트
            </span>
            <div
              class={center({
                opacity: '0',
                size: '16px',
                borderRadius: '4px',
                color: 'gray.400',
              })}
            >
              <Icon icon={EllipsisIcon} size={14} />
            </div>
          </div>

          <div
            class={cx(
              'group',
              css({
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
                paddingX: '8px',
                paddingY: '6px',
                borderRadius: '6px',
              }),
            )}
          >
            <div
              class={css({
                flex: 'none',
                borderRadius: 'full',
                backgroundColor: 'brand.500',
                size: '4px',
              })}
            ></div>
            <Icon style={css.raw({ color: 'gray.500' })} icon={FileIcon} size={14} />
            <span
              class={css({
                flexGrow: '1',
                fontSize: '14px',
                fontWeight: 'medium',
                color: 'gray.600',
                wordBreak: 'break-all',
                lineClamp: '1',
              })}
            >
              링크 공개 포스트
            </span>
            <div
              class={center({
                opacity: '0',
                size: '16px',
                borderRadius: '4px',
                color: 'gray.400',
              })}
            >
              <Icon icon={EllipsisIcon} size={14} />
            </div>
          </div>

          <div
            class={cx(
              'group',
              css({
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
                paddingX: '8px',
                paddingY: '6px',
                borderRadius: '6px',
              }),
            )}
          >
            <div
              class={css({
                flex: 'none',
                borderRadius: 'full',
                backgroundColor: 'gray.200',
                size: '4px',
              })}
            ></div>
            <Icon style={css.raw({ color: 'gray.500' })} icon={ShapesIcon} size={14} />
            <span
              class={css({
                flexGrow: '1',
                fontSize: '14px',
                fontWeight: 'medium',
                color: 'gray.600',
                wordBreak: 'break-all',
                lineClamp: '1',
              })}
            >
              일기 템플릿
            </span>
            <div
              class={center({
                opacity: '0',
                size: '16px',
                borderRadius: '4px',
                color: 'gray.400',
              })}
            >
              <Icon icon={EllipsisIcon} size={14} />
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 메인 콘텐츠 영역 -->
    <div class={css({ flexGrow: '1', display: 'flex', flexDirection: 'column', overflow: 'hidden' })}>
      <!-- 상단 헤더 -->
      <div
        class={flex({
          justifyContent: 'space-between',
          alignItems: 'center',
          gap: '6px',
          paddingLeft: '24px',
          paddingRight: '8px',
          height: '36px',
          borderBottomWidth: '1px',
          borderColor: 'gray.100',
        })}
      >
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          <Icon style={css.raw({ color: 'gray.400' })} icon={FolderIcon} size={12} />
          <div class={css({ flex: 'none', fontSize: '12px', color: 'gray.400' })}>내 포스트</div>
          <Icon style={css.raw({ color: 'gray.400' })} icon={ChevronRightIcon} size={12} />
          <div class={css({ fontSize: '12px', color: 'gray.400' })}>내 첫 폴더</div>
          <Icon style={css.raw({ color: 'gray.400' })} icon={ChevronRightIcon} size={12} />
          <div class={css({ flex: 'none', fontSize: '12px', fontWeight: 'medium', color: 'gray.700' })}>포스트 제목입니다</div>
        </div>

        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          <div class={center({ size: '24px' })}>
            <div style:background-color="#22c55e" class={css({ size: '8px', borderRadius: 'full' })}></div>
          </div>

          <div
            class={center({
              borderRadius: '4px',
              size: '24px',
              color: 'gray.500',
            })}
          >
            <Icon icon={EllipsisIcon} size={16} />
          </div>

          <div
            class={center({
              borderRadius: '4px',
              size: '24px',
              color: 'gray.500',
            })}
          >
            <Icon style={css.raw({ color: 'gray.500' })} icon={PanelRightCloseIcon} size={16} />
          </div>
        </div>
      </div>

      <!-- 툴바 -->
      <div
        class={css({
          display: 'flex',
          flexDirection: 'column',
          gap: '8px',
          borderBottomWidth: '1px',
          borderColor: 'gray.100',
          paddingY: '8px',
        })}
      >
        <div class={flex({ alignItems: 'center', gap: '4px', paddingX: '16px' })}>
          <div
            class={center({
              size: '32px',
              borderRadius: '4px',
              color: 'gray.500',
            })}
          >
            <Icon icon={ImageIcon} size={20} />
          </div>
          <div
            class={center({
              size: '32px',
              borderRadius: '4px',
              color: 'gray.500',
            })}
          >
            <Icon icon={PaperclipIcon} size={20} />
          </div>
          <div
            class={center({
              size: '32px',
              borderRadius: '4px',
              color: 'gray.500',
            })}
          >
            <Icon icon={FileUpIcon} size={20} />
          </div>
          <div
            class={center({
              size: '32px',
              borderRadius: '4px',
              color: 'gray.500',
            })}
          >
            <Icon icon={HorizontalRuleIcon} size={20} />
          </div>
          <div
            class={center({
              size: '32px',
              borderRadius: '4px',
              color: 'gray.500',
            })}
          >
            <Icon icon={QuoteIcon} size={20} />
          </div>
          <div
            class={center({
              size: '32px',
              borderRadius: '4px',
              color: 'gray.500',
            })}
          >
            <Icon icon={GalleryVerticalEndIcon} size={20} />
          </div>
          <div
            class={center({
              size: '32px',
              borderRadius: '4px',
              color: 'gray.500',
            })}
          >
            <Icon icon={TableIcon} size={20} />
          </div>
          <div
            class={center({
              size: '32px',
              borderRadius: '4px',
              color: 'gray.500',
            })}
          >
            <Icon icon={ListIcon} size={20} />
          </div>
          <div
            class={center({
              size: '32px',
              borderRadius: '4px',
              color: 'gray.500',
            })}
          >
            <Icon icon={CodeIcon} size={20} />
          </div>
          <div
            class={center({
              size: '32px',
              borderRadius: '4px',
              color: 'gray.500',
            })}
          >
            <Icon icon={CodeXmlIcon} size={20} />
          </div>
        </div>

        <div class={css({ width: 'full', height: '1px', backgroundColor: 'gray.100' })}></div>

        <div class={flex({ alignItems: 'center', gap: '10px', paddingLeft: '20px', paddingRight: '12px' })}>
          <div class={flex({ alignItems: 'center', gap: '4px' })}>
            <div
              class={center({
                size: '20px',
                borderRadius: '4px',
                borderRightRadius: '0',
                color: 'gray.500',
              })}
            >
              <Icon icon={UndoIcon} size={14} />
            </div>
            <div
              class={center({
                size: '20px',
                borderRadius: '4px',
                borderLeftRadius: '0',
                color: 'gray.500',
              })}
            >
              <Icon icon={RedoIcon} size={14} />
            </div>
          </div>

          <div class={css({ width: '1px', height: '12px', backgroundColor: 'gray.100' })}></div>

          <div class={flex({ alignItems: 'center', gap: '4px' })}>
            <div
              class={center({
                size: '20px',
                borderRadius: '4px',
                color: 'gray.500',
              })}
            >
              <div style:background-color="#000000" class={css({ borderWidth: '1px', borderRadius: 'full', size: '12px' })}></div>
            </div>
            <div
              class={center({
                size: '20px',
                borderRadius: '4px',
                color: 'gray.500',
              })}
            >
              <div
                style:background-color="transparent"
                class={css({
                  borderWidth: '1px',
                  borderRadius: '4px',
                  size: '12px',
                  position: 'relative',
                })}
              >
                <div
                  class={css({
                    position: 'absolute',
                    inset: '0',
                    margin: 'auto',
                    width: '1px',
                    height: '10px',
                    backgroundColor: 'gray.400',
                    transform: 'rotate(45deg)',
                  })}
                ></div>
              </div>
            </div>
          </div>

          <div class={css({ width: '1px', height: '12px', backgroundColor: 'gray.100' })}></div>

          <div class={flex({ alignItems: 'center', gap: '4px' })}>
            <div
              class={center({
                size: '20px',
                borderRadius: '4px',
                color: 'gray.700',
                backgroundColor: 'gray.200',
              })}
            >
              <Icon icon={BoldIcon} size={14} />
            </div>
            <div
              class={center({
                size: '20px',
                borderRadius: '4px',
                color: 'gray.500',
              })}
            >
              <Icon icon={ItalicIcon} size={14} />
            </div>
            <div
              class={center({
                size: '20px',
                borderRadius: '4px',
                color: 'gray.500',
              })}
            >
              <Icon icon={StrikethroughIcon} size={14} />
            </div>
            <div
              class={center({
                size: '20px',
                borderRadius: '4px',
                color: 'gray.500',
              })}
            >
              <Icon icon={UnderlineIcon} size={14} />
            </div>
          </div>

          <div class={css({ width: '1px', height: '12px', backgroundColor: 'gray.100' })}></div>

          <div class={flex({ alignItems: 'center', gap: '4px' })}>
            <div
              class={center({
                size: '20px',
                borderRadius: '4px',
                color: 'gray.500',
              })}
            >
              <Icon icon={LinkIcon} size={14} />
            </div>
          </div>

          <div class={css({ flexGrow: '1' })}></div>

          <div
            class={center({
              size: '20px',
              borderRadius: '4px',
              color: 'gray.500',
            })}
          >
            <Icon icon={CogIcon} size={14} />
          </div>
        </div>
      </div>

      <!-- 에디터 콘텐츠 영역 -->
      <div class={css({ flexGrow: '1', overflow: 'auto' })} data-element="main-content">
        <div
          class={flex({
            flexDirection: 'column',
            alignItems: 'center',
            paddingTop: '60px',
            paddingX: '80px',
            paddingBottom: '80px',
          })}
        >
          <div class={flex({ flexDirection: 'column', width: 'full', maxWidth: '600px' })}>
            <div class={css({ width: 'full', fontSize: '[1.5em]', fontWeight: 'bold', color: 'gray.900', marginBottom: '10px' })}>
              최후이자 최초의 너에게
            </div>

            <div
              class={css({
                width: 'full',
                height: '1px',
                backgroundColor: 'gray.100',
                marginBottom: '20px',
                willChange: 'width, height',
                transform: 'translateZ(0)',
              })}
              data-element="divider"
            ></div>
          </div>

          <div class={css({ width: 'full', maxWidth: '600px' })}>
            <div class={css({ marginX: 'auto', textIndent: '[1em]' })}>
              <p class={css({ marginBottom: '16px', lineHeight: '[1.6]', color: 'gray.900' })}>
                나는 그때를 새로운 별이 탄생하는 순간처럼 기억해. 내가 콜로니 한켠에 있는 커다란 구멍으로 바깥을 내다보고 있었고, 그 원의
                주변을 둘러 너를 기억하던 이들이 조화를 한 송이씩 내려놓던 그때. 블랙홀 같은 유리장을 빤히 바라보고 있자니 그 너머로 두터운
                우주복을 입은 사람 한 명이 나타났었어. 외벽에서 뻗어 나간 기계 팔에 매달려 조금씩 회전하다가는 우리를 바라보며 멈추었지.
                이윽고 스피커가 짧은 노이즈를 내다가 낮게 가라앉은 목소리로 변했어.
              </p>

              <p class={css({ marginBottom: '16px', lineHeight: '[1.6]', color: 'gray.900' })} data-element="body-text">
                "다들 이 자리에 함께해주셔서 고마워요."
              </p>

              <p class={css({ marginBottom: '16px', lineHeight: '[1.6]', color: 'gray.900' })} data-element="body-text">
                헬멧 안에서 입을 연 목소리의 주인은 너의 동생, 여울이었는데, 그 애는 그 한 마디만을 꺼내곤 오랫동안 말을 잇지 않았어. 차가운
                침묵이 식장을 관통하는 동안 우리 중 누구도 그 고요함에 작은 흠집 하나 내지 않았어. 여울이 입을 열었다가 닫았다가, 주저하며
                숨을 들이마시고는 다시 내뱉는 소리만이 스피커를 넘어 부서졌어.
              </p>

              <p class={css({ marginBottom: '16px', lineHeight: '[1.6]', color: 'gray.900' })} data-element="body-text">
                "아……. 그러니까, 솔직히, 제가 말재주가 없어서 무슨 말을 해야 하는 걸지는 잘 모르겠어요. 어젯밤부터 이 순간에 어떤 말을 하면
                좋을까 생각해 봤는데……. 할 수 있는 말이 달리 없을 것 같기도 하고, 굳이 많은 말을 할 필요도 없을 것 같아서. 그냥, 짧게 말하고
                마치려고요.
              </p>

              <p class={css({ marginBottom: '16px', lineHeight: '[1.6]', color: 'gray.900' })} data-element="body-text">
                세상에 영원한 건 없어요. 이 우주조차 영원하지는 않겠죠. 지나간 것들은 사라지고, 우리는 오직 기억만 할 수 있을 뿐이에요.
                그렇지만……. 그건 달리 말하면, 기억으로는 어떻게든 남길 수 있다는 뜻이라고도 생각하거든요. 지나간 존재가 그대로 사라져 버리는
                게 아니라 우리의 기억 속에 남아서. 우리가 기억하는 한, 각자의 기억 속에서, 각자가 알았던 모습이 되어 계속해서 존재할 수 있는
                거라고요. ……언니가 어떤 사람이었는가에 대해서는, 제 입에서 나오는 말이 아니라 여러분의 기억 속에 있는 모습으로 남기고
                싶어요. 그러니까……. 남은 시간 동안이라도 부디, 언니를 기억하고, 마저 사랑해주세요. ……그거면 충분할 거예요."
              </p>

              <p class={css({ marginBottom: '16px', lineHeight: '[1.6]', color: 'gray.900' })} data-element="body-text">
                슬픔은 언제나 균형을 흔드는 감정이니까. 말과 공백의 균형이 어그러져서, 고작 그것만으로 추도사는 끝났어. 그리고 여울은 손에
                쥔 구체를 조금씩 녹여나갔어. 그걸 반으로 쪼개어 나누자 안에 담겨 있던 잿빛의 분말들이 우주로 흩어지기 시작했고, 빛을 받아 꼭
                은하수가 흐르듯 빛났어. 모두 각자의 방향으로 뻗어 나가면서. 그러다가 어떤 너는 어느 별의 인력에 이끌려 버릴 수도 있을 테고,
                어떤 너는 우주를 떠도는 천체에 부딪혀 사라질 수도 있겠지. 그리고 어떤 너는 그대로 끝없이 우주를 뻗어 나갈지도 몰라. 어쩌면
                영원히.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 오른쪽 사이드바 -->
    <aside
      class={css({
        width: '240px',
        borderLeftWidth: '1px',
        borderColor: 'gray.100',
      })}
    >
      <div
        class={flex({
          flexDirection: 'column',
          gap: '16px',
          paddingTop: '16px',
          height: 'full',
        })}
      >
        <div class={flex({ flexDirection: 'column', gap: '6px', paddingX: '20px' })}>
          <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'gray.700' })}>포스트</div>
            <div class={cx('group', center({ size: '20px' }))}>
              <Icon style={css.raw({ color: 'gray.500' })} icon={ExternalLinkIcon} size={14} />
            </div>
          </div>
        </div>

        <div class={css({ width: 'full', height: '1px', backgroundColor: 'gray.100' })}></div>

        <div class={flex({ flexDirection: 'column', gap: '20px', paddingX: '20px' })}>
          <div class={flex({ flexDirection: 'column', gap: '6px' })}>
            <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
              <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'gray.700' })}>공유 및 게시</div>
              <div
                class={css({
                  fontSize: '13px',
                  fontWeight: 'medium',
                  color: 'gray.500',
                })}
              >
                설정
              </div>
            </div>
            <div
              class={css({
                borderRadius: '4px',
                paddingX: '8px',
                paddingY: '4px',
                width: 'fit',
                fontSize: '12px',
                fontWeight: 'semibold',
                color: 'gray.600',
                backgroundColor: 'gray.200',
                userSelect: 'none',
              })}
            >
              비공개
            </div>
          </div>

          <div class={flex({ flexDirection: 'column', gap: '6px' })}>
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'gray.700' })}>최초 생성 시각</div>
            <div class={css({ fontSize: '13px', color: 'gray.700' })}>2024년 1월 15일 오후 2:30</div>
          </div>

          <div class={flex({ flexDirection: 'column', gap: '6px' })}>
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'gray.700' })}>마지막 수정 시각</div>
            <div class={css({ fontSize: '13px', color: 'gray.700' })}>2024년 1월 15일 오후 5:45</div>
          </div>

          <div class={flex({ flexDirection: 'column', gap: '12px' })}>
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'gray.700' })}>본문 정보</div>

            <div class={flex({ flexDirection: 'column' })}>
              <details class={flex({ flexDirection: 'column', marginBottom: '8px' })}>
                <summary class={flex({ alignItems: 'center', gap: '4px', cursor: 'pointer', userSelect: 'none' })}>
                  <Icon style={{ color: 'gray.500' }} icon={TypeIcon} size={12} />
                  <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.700' })}>글자 수</div>
                  <Icon style={css.raw({ color: 'gray.500' })} icon={ChevronRightIcon} size={14} />
                  <div class={css({ flexGrow: '1' })}></div>
                  <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.700' })}>523자</div>
                </summary>
              </details>

              <details class={flex({ flexDirection: 'column', marginBottom: '0' })}>
                <summary class={flex({ alignItems: 'center', gap: '4px', cursor: 'pointer', userSelect: 'none' })}>
                  <Icon style={{ color: 'gray.500' }} icon={GoalIcon} size={12} />
                  <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.700' })}>오늘의 기록</div>
                  <Icon style={css.raw({ color: 'gray.500' })} icon={ChevronRightIcon} size={14} />
                  <div class={css({ flexGrow: '1' })}></div>
                  <div class={flex({ alignItems: 'center', gap: '4px', fontSize: '13px', fontWeight: 'medium', color: 'gray.700' })}>
                    <Icon style={css.raw({ color: 'gray.500' })} icon={TrendingUpIcon} size={14} />
                    <span>+156자</span>
                  </div>
                </summary>
              </details>
            </div>
          </div>
        </div>

        <div class={css({ width: 'full', height: '1px', backgroundColor: 'gray.100' })}></div>

        <div class={flex({ flexDirection: 'column', gap: '8px', paddingX: '20px', flexGrow: '1' })}>
          <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'gray.700' })}>노트</div>
          <div
            class={css({
              fontSize: '13px',
              lineHeight: '[1.6]',
              color: 'gray.600',
              borderRadius: '8px',
              padding: '12px',
              backgroundColor: 'gray.50',
              minHeight: '80px',
            })}
          >
            여기에 메모나 아이디어를 적어보세요...
          </div>
        </div>
      </div>
    </aside>
  </div>
</div>
