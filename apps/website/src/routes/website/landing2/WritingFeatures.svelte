<script lang="ts">
  import AlignLeftIcon from '~icons/lucide/align-left';
  import BlendIcon from '~icons/lucide/blend';
  import BoldIcon from '~icons/lucide/bold';
  import BookmarkIcon from '~icons/lucide/bookmark';
  import CheckIcon from '~icons/lucide/check';
  import ChevronsDownUpIcon from '~icons/lucide/chevrons-down-up';
  import CircleIcon from '~icons/lucide/circle';
  import CodeIcon from '~icons/lucide/code';
  import CodeXmlIcon from '~icons/lucide/code-xml';
  import DraftingCompassIcon from '~icons/lucide/drafting-compass';
  import FileUpIcon from '~icons/lucide/file-up';
  import GalleryVerticalEndIcon from '~icons/lucide/gallery-vertical-end';
  import ImageIcon from '~icons/lucide/image';
  import ItalicIcon from '~icons/lucide/italic';
  import LinkIcon from '~icons/lucide/link';
  import ListIcon from '~icons/lucide/list';
  import MousePointer2Icon from '~icons/lucide/mouse-pointer-2';
  import NotebookTabsIcon from '~icons/lucide/notebook-tabs';
  import PaletteIcon from '~icons/lucide/palette';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import PencilIcon from '~icons/lucide/pencil';
  import PlusIcon from '~icons/lucide/plus';
  import QuoteIcon from '~icons/lucide/quote';
  import SettingsIcon from '~icons/lucide/settings';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import SquareIcon from '~icons/lucide/square';
  import StrikethroughIcon from '~icons/lucide/strikethrough';
  import TableIcon from '~icons/lucide/table';
  import TypeIcon from '~icons/lucide/type';
  import UnderlineIcon from '~icons/lucide/underline';
  import HorizontalRuleIcon from '~icons/typie/horizontal-rule';
  import LetterSpacingIcon from '~icons/typie/letter-spacing';
  import LineHeightIcon from '~icons/typie/line-height';
  import RubyIcon from '~icons/typie/ruby';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex, grid } from '$styled-system/patterns';
  import Bento from './Bento.svelte';

  const fontExamples = [
    { font: 'Pretendard', text: '프리텐다드' },
    { font: 'KoPubWorldDotum', text: '코펍월드돋움' },
    { font: 'NanumBarunGothic', text: '나눔바른고딕' },
    { font: 'RIDIBatang', text: '리디바탕' },
    { font: 'KoPubWorldBatang', text: '코펍월드바탕' },
    { font: 'NanumMyeongjo', text: '나눔명조' },
  ];

  const editorTools = [
    { icon: ImageIcon },
    { icon: PaperclipIcon },
    { icon: FileUpIcon },
    { icon: HorizontalRuleIcon },
    { icon: QuoteIcon },
    { icon: GalleryVerticalEndIcon },
    { icon: ChevronsDownUpIcon },
    { icon: TableIcon },
    { icon: ListIcon },
    { icon: CodeIcon },
    { icon: CodeXmlIcon },
    { icon: BoldIcon },
    { icon: ItalicIcon },
    { icon: StrikethroughIcon },
    { icon: UnderlineIcon },
    { icon: LinkIcon },
    { icon: RubyIcon },
    { icon: AlignLeftIcon },
    { icon: LineHeightIcon },
    { icon: LetterSpacingIcon },
    { icon: BlendIcon },
    { icon: SpellCheckIcon },
    { icon: BookmarkIcon },
    { icon: NotebookTabsIcon },
  ];
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
      <Icon icon={DraftingCompassIcon} size={16} />
      완벽한 글쓰기 도구
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
      글쓰기에 필요한 모든 도구
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
      본문 작성부터 아이디어 시각화까지, 글쓰기의 모든 과정을 지원합니다.
      <br />
      생각을 온전히 표현할 수 있는 완벽한 도구들이 준비되어 있습니다.
    </p>
  </div>

  <Bento>
    <div class={center({ flexGrow: 1 })}>
      <div class={css({ position: 'relative', height: '[220px]', overflow: 'hidden' })}>
        <div class={grid({ columns: 8, gap: '20px' })}>
          {#each editorTools as tool (tool.icon)}
            <div
              class={css({
                size: '64px',
                borderRadius: '12px',
                backgroundColor: 'white',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                boxShadow: 'small',
              })}
            >
              <Icon style={css.raw({ color: 'gray.700' })} icon={tool.icon} size={28} />
            </div>
          {/each}

          <div
            class={css({
              size: '64px',
              borderRadius: '12px',
              backgroundColor: 'white',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              boxShadow: 'small',
            })}
          >
            <div class={css({ size: '28px', borderRadius: 'full', backgroundColor: 'red.500' })}></div>
          </div>
          <div
            class={css({
              size: '64px',
              borderRadius: '12px',
              backgroundColor: 'white',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              boxShadow: 'small',
            })}
          >
            <div class={css({ size: '28px', borderRadius: '6px', backgroundColor: 'amber.200' })}></div>
          </div>
        </div>

        <div
          class={css({
            position: 'absolute',
            bottom: '0',
            left: '0',
            right: '0',
            height: '60px',
            background: '[linear-gradient(to top, rgba(255, 255, 255, 1) 0%, rgba(255, 255, 255, 0.9) 30%, rgba(255, 255, 255, 0) 100%)]',
            pointerEvents: 'none',
          })}
        ></div>
      </div>
    </div>

    <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
      <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
        <Icon icon={DraftingCompassIcon} size={20} />
        <h3 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'gray.800' })}>글쓰기에 필요한 모든 것</h3>
      </div>
      <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]' })}>
        기본적인 서식부터 이미지, 링크, 표 삽입까지. 글을 풍부하게 만드는 데 필요한 모든 도구가 준비되어 있습니다.
      </p>
    </div>
  </Bento>

  <div
    class={flex({
      gap: '24px',
      marginTop: '24px',
    })}
  >
    <div class={css({ flex: '1', display: 'flex', flexDirection: 'column', gap: '24px' })}>
      <Bento>
        <div class={css({ position: 'relative', flexGrow: 1, overflow: 'hidden' })}>
          <div
            class={css({
              position: 'relative',
              width: 'full',
              height: 'full',
              backgroundColor: 'gray.50',
              borderRadius: '8px',
              overflow: 'hidden',
              backgroundImage: `
                linear-gradient(to right, rgba(0, 0, 0, 0.05) 1px, transparent 1px),
                linear-gradient(to bottom, rgba(0, 0, 0, 0.05) 1px, transparent 1px)
              `,
              backgroundSize: '20px 20px',
            })}
          >
            <div
              class={css({
                position: 'absolute',
                top: '[50%]',
                left: '[50%]',
                transform: 'translate(-50%, -50%) translate(-40px, -20px)',
                width: '140px',
                height: '140px',
                borderRadius: '[20px]',
                backgroundColor: '[#dbeafe]',
                borderWidth: '2px',
                borderColor: '[#60a5fa]',
                boxShadow: 'small',
              })}
            ></div>

            <div
              class={css({
                position: 'absolute',
                top: '[50%]',
                left: '[50%]',
                transform: 'translate(-50%, -50%) translate(40px, 20px)',
                width: '120px',
                height: '120px',
                borderRadius: 'full',
                backgroundColor: '[#fef3c7]',
                borderWidth: '2px',
                borderColor: '[#fbbf24]',
                boxShadow: 'small',
              })}
            ></div>

            <div
              class={flex({
                position: 'absolute',
                bottom: '10px',
                left: '1/2',
                transform: 'translateX(-50%)',
                gap: '4px',
                padding: '6px',
                backgroundColor: 'white',
                borderRadius: '8px',
                boxShadow: 'small',
              })}
            >
              <div
                class={center({
                  size: '24px',
                  borderRadius: '4px',
                  backgroundColor: 'gray.100',
                  color: 'gray.600',
                })}
              >
                <Icon icon={MousePointer2Icon} size={14} />
              </div>
              <div
                class={center({
                  size: '24px',
                  borderRadius: '4px',
                  backgroundColor: 'amber.100',
                  color: 'amber.700',
                })}
              >
                <Icon icon={SquareIcon} size={14} />
              </div>
              <div
                class={center({
                  size: '24px',
                  borderRadius: '4px',
                  backgroundColor: 'gray.100',
                  color: 'gray.600',
                })}
              >
                <Icon icon={CircleIcon} size={14} />
              </div>
              <div class={css({ width: '1px', height: '20px', backgroundColor: 'gray.200', marginX: '2px' })}></div>
              <div
                class={center({
                  size: '24px',
                  borderRadius: '4px',
                  backgroundColor: 'gray.100',
                  color: 'gray.600',
                })}
              >
                <Icon icon={PencilIcon} size={14} />
              </div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={PaletteIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'gray.800' })}>캔버스</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]' })}>
            자유롭게 그림을 그리고 다이어그램을 만들어 보세요. 아이디어를 시각적으로 표현하고 글과 함께 배치할 수 있습니다.
          </p>
        </div>
      </Bento>

      <Bento>
        <div class={center({ flexGrow: 1 })}>
          <div class={css({ width: 'full', maxWidth: '320px' })}>
            <div
              class={css({
                padding: '20px',
                borderRadius: '8px',
                backgroundColor: 'white',
                border: '[1px solid]',
                borderColor: 'gray.200',
                marginBottom: '16px',
                position: 'relative',
              })}
            >
              <p class={css({ fontSize: '15px', lineHeight: '[1.8]', color: 'gray.800' })}>
                겨울밤 하늘에는 별들이
                <span
                  class={css({
                    textDecoration: 'underline wavy',
                    textDecorationColor: 'red.500',
                    textUnderlineOffset: '2px',
                  })}
                >
                  총총이 빛나고
                </span>
                있었다. 찬바람이 불어올 때마다 창문이 덜컹거렸다.
              </p>
            </div>

            <div
              class={css({
                padding: '8px',
                borderRadius: '8px',
                backgroundColor: 'white',
                boxShadow: 'medium',
                border: '[1px solid]',
                borderColor: 'gray.200',
              })}
            >
              <button
                class={flex({
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  gap: '8px',
                  borderWidth: '1px',
                  borderColor: 'red.200',
                  borderRadius: '6px',
                  paddingX: '12px',
                  paddingY: '8px',
                  fontSize: '14px',
                  fontWeight: 'medium',
                  color: 'red.700',
                  backgroundColor: 'red.50',
                  width: 'full',
                  _hover: {
                    backgroundColor: 'red.100',
                  },
                })}
                type="button"
              >
                <span>총총히 빛나고</span>
                <Icon icon={CheckIcon} size={16} />
              </button>

              <div
                class={css({
                  marginTop: '8px',
                  paddingX: '12px',
                  paddingY: '8px',
                  borderRadius: '6px',
                  backgroundColor: 'gray.50',
                  fontSize: '12px',
                  color: 'gray.600',
                  lineHeight: '[1.5]',
                })}
              >
                <strong class={css({ color: 'gray.700' })}>부사 표기 오류</strong>
                <br />
                '총총이'는 '총총히'로 써야 올바른 표현입니다.
              </div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={SpellCheckIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'gray.800' })}>정교한 맞춤법 검사</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]' })}>
            오탈자 걱정 없이 편하게 쓸 수 있도록, 맞춤법과 문법을 꼼꼼하게 살펴 교정 방향과 그 이유를 함께 제시합니다.
          </p>
        </div>
      </Bento>
    </div>

    <div class={css({ flex: '1', display: 'flex', flexDirection: 'column', gap: '24px' })}>
      <Bento>
        <div class={center({ flexGrow: '1' })}>
          <div class={flex({ gap: '16px', alignItems: 'stretch', flexGrow: '1' })}>
            <div
              class={css({
                flexGrow: '1',
                borderWidth: '1px',
                borderColor: 'gray.200',
                borderRadius: '8px',
                backgroundColor: 'white',
                overflow: 'hidden',
                boxShadow: 'small',
              })}
            >
              {#each fontExamples as example, index (example.font)}
                <div
                  class={css({
                    paddingX: '16px',
                    paddingY: '12px',
                    borderBottomWidth: index < fontExamples.length - 1 ? '1px' : '0',
                    borderColor: 'gray.100',
                  })}
                >
                  <p
                    style:font-family={example.font}
                    class={css({
                      fontSize: '14px',
                      lineHeight: '[1.4]',
                      color: 'gray.800',
                      margin: '0',
                    })}
                  >
                    {example.text}
                  </p>
                </div>
              {/each}
            </div>

            <div
              class={css({
                flexGrow: '1',
                padding: '20px',
                backgroundColor: 'amber.50',
                borderWidth: '2px',
                borderStyle: 'dashed',
                borderColor: 'amber.300',
                borderRadius: '8px',
                transition: 'common',
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '12px',
                _hover: {
                  borderColor: 'amber.400',
                  backgroundColor: 'amber.100',
                },
              })}
            >
              <Icon style={css.raw({ color: 'amber.600' })} icon={PlusIcon} size={24} />
              <div class={css({ textAlign: 'center' })}>
                <p class={css({ fontSize: '13px', fontWeight: 'medium', color: 'amber.700', marginBottom: '4px' })}>
                  내가 원하는
                  <br />
                  폰트 업로드
                </p>
                <p class={css({ fontSize: '11px', color: 'amber.600' })}>TTF, OTF 파일 지원</p>
              </div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={TypeIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'gray.800' })}>자유로운 폰트 선택</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]' })}>
            엄선된 기본 폰트는 물론, 원하는 폰트를 직접 업로드하여 원하는 모습으로 글을 꾸밀 수 있습니다.
          </p>
        </div>
      </Bento>

      <Bento>
        <div class={center({ flexGrow: 1 })}>
          <div class={flex({ flexDirection: 'column', gap: '12px', width: 'full', maxWidth: '400px' })}>
            <div
              class={css({
                padding: '20px',
                borderRadius: '8px',
                backgroundColor: 'white',
                border: '[1px solid]',
                borderColor: 'gray.200',
              })}
            >
              <p class={css({ fontSize: '15px', lineHeight: '[1.7]', color: 'gray.800' })}>
                봄비가 내리던 날, 나는 낡은 우산을 들고 거리를 걸었다. 빗방울이 우산에 부딪히는 소리가 마치 옛 추억을 두드리는 것 같았다.
                젖은 아스팔트 위로 가로등 불빛이 번지고 있었다.
              </p>
            </div>

            <div
              class={css({
                padding: '16px',
                borderRadius: '8px',
                backgroundColor: 'amber.50',
                border: '[1px solid]',
                borderColor: 'amber.200',
              })}
            >
              <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'amber.700', marginBottom: '8px' })}>작성 노트</div>
              <div
                class={css({
                  fontSize: '13px',
                  color: 'gray.700',
                  lineHeight: '[1.5]',
                })}
              >
                우산 소리 묘사 더 구체적으로 / 주인공 감정선 추가 / 거리 풍경 세밀하게 / 다음 장면 전환 고민
              </div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={NotebookTabsIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'gray.800' })}>작성 노트</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]' })}>
            쓰기 중 떠오르는 아이디어나 메모를 포스트 옆에 바로 기록할 수 있어, 흐름을 유지하며 생각을 보관할 수 있습니다.
          </p>
        </div>
      </Bento>

      <Bento>
        <div class={center({ flexGrow: 1 })}>
          <div
            class={flex({
              flexDirection: 'column',
              gap: '20px',
              borderWidth: '1px',
              borderColor: 'gray.200',
              borderRadius: '8px',
              padding: '20px',
            })}
          >
            <div class={css({ width: 'full' })}>
              <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.600', marginBottom: '10px' })}>본문 폭</div>
              <div class={flex({ alignItems: 'center', gap: '12px' })}>
                <div
                  class={css({
                    paddingX: '16px',
                    paddingY: '6px',
                    borderRadius: 'full',
                    backgroundColor: 'gray.50',
                    fontSize: '13px',
                    fontWeight: 'medium',
                    color: 'gray.400',
                    opacity: '40',
                  })}
                >
                  600px
                </div>
                <div
                  class={css({
                    paddingX: '16px',
                    paddingY: '6px',
                    borderRadius: 'full',
                    backgroundColor: 'amber.100',
                    borderWidth: '1px',
                    borderColor: 'amber.300',
                    fontSize: '13px',
                    fontWeight: 'medium',
                    color: 'amber.700',
                  })}
                >
                  800px
                </div>
                <div
                  class={css({
                    paddingX: '16px',
                    paddingY: '6px',
                    borderRadius: 'full',
                    backgroundColor: 'gray.50',
                    fontSize: '13px',
                    fontWeight: 'medium',
                    color: 'gray.400',
                    opacity: '40',
                  })}
                >
                  1000px
                </div>
              </div>
            </div>

            <div class={css({ width: 'full' })}>
              <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.600', marginBottom: '10px' })}>첫 줄 들여쓰기</div>
              <div class={flex({ alignItems: 'center', gap: '12px' })}>
                <div
                  class={css({
                    paddingX: '16px',
                    paddingY: '6px',
                    borderRadius: 'full',
                    backgroundColor: 'amber.100',
                    borderWidth: '1px',
                    borderColor: 'amber.300',
                    fontSize: '13px',
                    fontWeight: 'medium',
                    color: 'amber.700',
                  })}
                >
                  없음
                </div>
                <div
                  class={css({
                    paddingX: '16px',
                    paddingY: '6px',
                    borderRadius: 'full',
                    backgroundColor: 'gray.50',
                    fontSize: '13px',
                    fontWeight: 'medium',
                    color: 'gray.400',
                    opacity: '40',
                  })}
                >
                  0.5칸
                </div>
                <div
                  class={css({
                    paddingX: '16px',
                    paddingY: '6px',
                    borderRadius: 'full',
                    backgroundColor: 'gray.50',
                    fontSize: '13px',
                    fontWeight: 'medium',
                    color: 'gray.400',
                    opacity: '40',
                  })}
                >
                  1칸
                </div>
                <div
                  class={css({
                    paddingX: '16px',
                    paddingY: '6px',
                    borderRadius: 'full',
                    backgroundColor: 'gray.50',
                    fontSize: '13px',
                    fontWeight: 'medium',
                    color: 'gray.400',
                    opacity: '40',
                  })}
                >
                  2칸
                </div>
              </div>
            </div>

            <div class={css({ width: 'full' })}>
              <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.600', marginBottom: '10px' })}>문단 사이 간격</div>
              <div class={flex({ alignItems: 'center', gap: '12px' })}>
                <div
                  class={css({
                    paddingX: '16px',
                    paddingY: '6px',
                    borderRadius: 'full',
                    backgroundColor: 'gray.50',
                    fontSize: '13px',
                    fontWeight: 'medium',
                    color: 'gray.400',
                    opacity: '40',
                  })}
                >
                  없음
                </div>
                <div
                  class={css({
                    paddingX: '16px',
                    paddingY: '6px',
                    borderRadius: 'full',
                    backgroundColor: 'gray.50',
                    fontSize: '13px',
                    fontWeight: 'medium',
                    color: 'gray.400',
                    opacity: '40',
                  })}
                >
                  0.5줄
                </div>
                <div
                  class={css({
                    paddingX: '16px',
                    paddingY: '6px',
                    borderRadius: 'full',
                    backgroundColor: 'amber.100',
                    borderWidth: '1px',
                    borderColor: 'amber.300',
                    fontSize: '13px',
                    fontWeight: 'medium',
                    color: 'amber.700',
                  })}
                >
                  1줄
                </div>
                <div
                  class={css({
                    paddingX: '16px',
                    paddingY: '6px',
                    borderRadius: 'full',
                    backgroundColor: 'gray.50',
                    fontSize: '13px',
                    fontWeight: 'medium',
                    color: 'gray.400',
                    opacity: '40',
                  })}
                >
                  2줄
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={SettingsIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'gray.800' })}>원고 양식 설정</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]' })}>
            본문 폭, 들여쓰기, 문단 사이 간격 등 세밀한 양식 설정으로 가장 읽기 편한 환경을 만들 수 있습니다.
          </p>
        </div>
      </Bento>
    </div>
  </div>
</section>
