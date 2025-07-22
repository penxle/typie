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
      <Icon icon={DraftingCompassIcon} size={16} />
      완벽한 글쓰기 도구
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
      글쓰기에 필요한
      <br />
      <span
        class={css({
          backgroundColor: 'amber.400',
          paddingX: '20px',
          display: 'inline-block',
          transform: 'rotate(1deg)',
        })}
      >
        모든 도구
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
      본문 작성부터 아이디어 시각화까지, 글쓰기의 모든 과정을 지원합니다.
      <br />
      생각을 온전히 표현할 수 있는 완벽한 도구들이 준비되어 있습니다.
    </p>
  </div>

  <Bento>
    <div class={center({ flexGrow: 1 })}>
      <div class={css({ position: 'relative', height: '[220px]', overflow: 'hidden' })}>
        <div class={grid({ columns: 8, gap: '16px' })}>
          {#each editorTools as tool (tool.icon)}
            <div
              class={css({
                size: '56px',
                backgroundColor: 'white',
                border: '3px solid',
                borderColor: 'gray.900',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition:
                  '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), background-color 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                _hover: {
                  backgroundColor: 'amber.400',
                  transform: 'translateY(-2px)',
                },
              })}
            >
              <Icon style={css.raw({ color: 'gray.900' })} icon={tool.icon} size={24} />
            </div>
          {/each}

          <div
            class={css({
              size: '56px',
              backgroundColor: 'white',
              border: '3px solid',
              borderColor: 'gray.900',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
              _hover: {
                transform: 'translateY(-2px)',
              },
            })}
          >
            <div class={css({ size: '24px', borderRadius: 'full', backgroundColor: 'red.500' })}></div>
          </div>
          <div
            class={css({
              size: '56px',
              backgroundColor: 'white',
              border: '3px solid',
              borderColor: 'gray.900',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
              _hover: {
                transform: 'translateY(-2px)',
              },
            })}
          >
            <div class={css({ size: '24px', backgroundColor: 'amber.400' })}></div>
          </div>
        </div>

        <div
          class={css({
            position: 'absolute',
            bottom: '0',
            left: '0',
            right: '0',
            height: '40px',
            background: '[linear-gradient(to top, rgba(255, 255, 255, 1) 0%, rgba(255, 255, 255, 0.8) 50%, rgba(255, 255, 255, 0) 100%)]',
            pointerEvents: 'none',
          })}
        ></div>
      </div>
    </div>

    <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
      <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
        <Icon icon={DraftingCompassIcon} size={20} />
        <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>
          글쓰기에 필요한 모든 것
        </h3>
      </div>
      <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
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
                backgroundColor: 'blue.200',
                border: '4px solid',
                borderColor: 'blue.600',
                boxShadow: '[4px 4px 0 0 #000]',
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
                backgroundColor: 'amber.200',
                border: '4px solid',
                borderColor: 'amber.600',
                boxShadow: '[4px 4px 0 0 #000]',
              })}
            ></div>

            <div
              class={flex({
                position: 'absolute',
                bottom: '10px',
                left: '1/2',
                transform: 'translateX(-50%)',
                gap: '4px',
                padding: '8px',
                backgroundColor: 'white',
                border: '3px solid',
                borderColor: 'gray.900',
                boxShadow: '[3px 3px 0 0 #000]',
              })}
            >
              <div
                class={center({
                  size: '28px',
                  backgroundColor: 'gray.100',
                  color: 'gray.600',
                  border: '2px solid',
                  borderColor: 'gray.900',
                  transition: 'all',
                  transitionDuration: '150ms',
                  _hover: {
                    backgroundColor: 'gray.900',
                    color: 'white',
                  },
                })}
              >
                <Icon icon={MousePointer2Icon} size={16} />
              </div>
              <div
                class={center({
                  size: '28px',
                  backgroundColor: 'amber.400',
                  color: 'gray.900',
                  border: '2px solid',
                  borderColor: 'gray.900',
                })}
              >
                <Icon icon={SquareIcon} size={16} />
              </div>
              <div
                class={center({
                  size: '28px',
                  backgroundColor: 'gray.100',
                  color: 'gray.600',
                  border: '2px solid',
                  borderColor: 'gray.900',
                  transition: 'all',
                  transitionDuration: '150ms',
                  _hover: {
                    backgroundColor: 'gray.900',
                    color: 'white',
                  },
                })}
              >
                <Icon icon={CircleIcon} size={16} />
              </div>
              <div class={css({ width: '2px', height: '24px', backgroundColor: 'gray.900', marginX: '2px' })}></div>
              <div
                class={center({
                  size: '28px',
                  backgroundColor: 'gray.100',
                  color: 'gray.600',
                  border: '2px solid',
                  borderColor: 'gray.900',
                  transition: 'all',
                  transitionDuration: '150ms',
                  _hover: {
                    backgroundColor: 'gray.900',
                    color: 'white',
                  },
                })}
              >
                <Icon icon={PencilIcon} size={16} />
              </div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={PaletteIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>캔버스</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
            자유롭게 그림을 그리고 다이어그램을 만들어 보세요. 아이디어를 시각적으로 표현하고 글과 함께 배치할 수 있습니다.
          </p>
        </div>
      </Bento>

      <Bento>
        <div class={center({ flexGrow: 1 })}>
          <div class={css({ width: 'full', maxWidth: '360px' })}>
            <div
              class={css({
                padding: '20px',
                backgroundColor: 'white',
                border: '4px solid',
                borderColor: 'gray.900',
                marginBottom: '16px',
                position: 'relative',
                boxShadow: '[4px 4px 0 0 #000]',
              })}
            >
              <p class={css({ fontSize: '15px', lineHeight: '[1.8]', color: 'gray.800', fontWeight: 'medium' })}>
                겨울밤 하늘에는 별들이
                <span
                  class={css({
                    textDecoration: 'underline wavy',
                    textDecorationColor: 'red.500',
                    textUnderlineOffset: '4px',
                    textDecorationThickness: '2px',
                  })}
                >
                  총총이 빛나고
                </span>
                있었다. 찬바람이 불어올 때마다 창문이 덜컹거렸다.
              </p>
            </div>

            <div
              class={css({
                padding: '12px',
                backgroundColor: 'white',
                border: '4px solid',
                borderColor: 'gray.900',
                boxShadow: '[4px 4px 0 0 #000]',
              })}
            >
              <button
                class={flex({
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  gap: '8px',
                  border: '3px solid',
                  borderColor: 'red.500',
                  paddingX: '16px',
                  paddingY: '10px',
                  fontSize: '14px',
                  fontWeight: 'bold',
                  color: 'red.700',
                  backgroundColor: 'red.50',
                  width: 'full',
                  transition: 'all',
                  transitionDuration: '150ms',
                  _hover: {
                    backgroundColor: 'red.100',
                    transform: 'translateY(-2px)',
                  },
                })}
                type="button"
              >
                <span>총총히 빛나고</span>
                <Icon icon={CheckIcon} size={16} />
              </button>

              <div
                class={css({
                  marginTop: '12px',
                  paddingX: '16px',
                  paddingY: '12px',
                  backgroundColor: 'gray.50',
                  fontSize: '13px',
                  color: 'gray.600',
                  lineHeight: '[1.5]',
                  border: '3px solid',
                  borderColor: 'gray.300',
                  fontWeight: 'medium',
                })}
              >
                <strong class={css({ color: 'gray.800' })}>부사 표기 오류</strong>
                <br />
                '총총이'는 '총총히'로 써야 올바른 표현입니다.
              </div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={SpellCheckIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>
              정교한 맞춤법 검사
            </h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
            오탈자 걱정 없이 편하게 쓸 수 있도록, 맞춤법과 문법을 꼼꼼하게 살펴 교정 방향과 그 이유를 함께 제시합니다.
          </p>
        </div>
      </Bento>
    </div>

    <div class={css({ flex: '1', display: 'flex', flexDirection: 'column', gap: '24px' })}>
      <Bento>
        <div class={center({ flexGrow: '1' })}>
          <div class={flex({ gap: '16px', alignItems: 'stretch', flexGrow: '1', maxWidth: '380px' })}>
            <div
              class={css({
                flexGrow: '1',
                border: '4px solid',
                borderColor: 'gray.900',
                backgroundColor: 'white',
                overflow: 'hidden',
                boxShadow: '[4px 4px 0 0 #000]',
              })}
            >
              {#each fontExamples as example, index (example.font)}
                <div
                  class={css({
                    paddingX: '16px',
                    paddingY: '14px',
                    borderBottomWidth: index < fontExamples.length - 1 ? '3px' : '0',
                    borderColor: 'gray.900',
                    transition: 'all',
                    transitionDuration: '150ms',
                    _hover: {
                      backgroundColor: 'amber.50',
                    },
                  })}
                >
                  <p
                    style:font-family={example.font}
                    class={css({
                      fontSize: '15px',
                      lineHeight: '[1.4]',
                      color: 'gray.800',
                      margin: '0',
                      fontWeight: 'semibold',
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
                border: '4px dashed',
                borderColor: 'amber.600',
                transition: 'all',
                transitionDuration: '200ms',
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '12px',
                cursor: 'pointer',
                _hover: {
                  borderColor: 'gray.900',
                  backgroundColor: 'amber.100',
                  borderStyle: 'solid',
                },
              })}
            >
              <Icon style={css.raw({ color: 'amber.700' })} icon={PlusIcon} size={28} />
              <div class={css({ textAlign: 'center' })}>
                <p class={css({ fontSize: '14px', fontWeight: 'bold', color: 'amber.800', marginBottom: '4px' })}>
                  내가 원하는
                  <br />
                  폰트 업로드
                </p>
                <p class={css({ fontSize: '12px', color: 'amber.700', fontWeight: 'medium' })}>TTF, OTF 파일 지원</p>
              </div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={TypeIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>
              자유로운 폰트 선택
            </h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
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
                backgroundColor: 'white',
                border: '4px solid',
                borderColor: 'gray.900',
                boxShadow: '[4px 4px 0 0 #000]',
              })}
            >
              <p class={css({ fontSize: '15px', lineHeight: '[1.7]', color: 'gray.800', fontWeight: 'medium' })}>
                봄비가 내리던 날, 나는 낡은 우산을 들고 거리를 걸었다. 빗방울이 우산에 부딪히는 소리가 마치 옛 추억을 두드리는 것 같았다.
                젖은 아스팔트 위로 가로등 불빛이 번지고 있었다.
              </p>
            </div>

            <div
              class={css({
                padding: '16px',
                backgroundColor: 'amber.400',
                border: '4px solid',
                borderColor: 'gray.900',
                position: 'relative',
                transform: 'rotate(1deg)',
                boxShadow: '[4px 4px 0 0 #000]',
                transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                _hover: {
                  transform: 'rotate(0)',
                },
              })}
            >
              <div
                class={css({
                  position: 'absolute',
                  top: '-16px',
                  right: '16px',
                  backgroundColor: 'gray.900',
                  color: 'white',
                  paddingX: '12px',
                  paddingY: '4px',
                  fontSize: '12px',
                  fontWeight: 'black',
                  textTransform: 'uppercase',
                  letterSpacing: '[0.1em]',
                  border: '3px solid',
                  borderColor: 'gray.900',
                  boxShadow: '[3px 3px 0 0 #fbbf24]',
                })}
              >
                작성 노트
              </div>
              <p class={css({ fontSize: '14px', lineHeight: '[1.6]', color: 'gray.900', fontWeight: 'bold' })}>
                - 비 오는 날의 정서를 더 강조
                <br />
                - 우산의 의미 확장 필요
                <br />
                - 마지막에 희망적 메시지 추가?
              </p>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={NotebookTabsIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>작성 노트</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
            쓰기 중 떠오르는 아이디어나 메모를 포스트 옆에 바로 기록할 수 있습니다.
          </p>
        </div>
      </Bento>
    </div>
  </div>

  <div
    class={flex({
      gap: '24px',
      marginTop: '24px',
    })}
  >
    <div class={css({ flex: '1' })}>
      <Bento>
        <div class={center({ flexGrow: 1 })}>
          <div class={css({ width: 'full', maxWidth: '380px' })}>
            <div
              class={flex({
                alignItems: 'center',
                gap: '16px',
                paddingX: '20px',
                paddingY: '16px',
                backgroundColor: 'gray.50',
                border: '4px solid',
                borderColor: 'gray.900',
                marginBottom: '16px',
              })}
            >
              <div class={css({ fontSize: '14px', fontWeight: 'bold', color: 'gray.600', textTransform: 'uppercase' })}>본문 폭</div>
              <div
                class={css({
                  flex: '1',
                  height: '6px',
                  backgroundColor: 'gray.300',
                  position: 'relative',
                })}
              >
                <div
                  class={css({
                    position: 'absolute',
                    left: '0',
                    width: '[60%]',
                    height: 'full',
                    backgroundColor: 'gray.900',
                  })}
                ></div>
                <div
                  class={css({
                    position: 'absolute',
                    left: '[60%]',
                    top: '[50%]',
                    transform: 'translate(-50%, -50%)',
                    size: '20px',
                    backgroundColor: 'amber.400',
                    border: '4px solid',
                    borderColor: 'gray.900',
                    boxShadow: '[2px 2px 0 0 #000]',
                    cursor: 'pointer',
                  })}
                ></div>
              </div>
              <div class={css({ fontSize: '14px', fontWeight: 'black', color: 'gray.900' })}>600px</div>
            </div>

            <div
              class={css({
                padding: '20px',
                backgroundColor: 'white',
                border: '4px solid',
                borderColor: 'gray.900',
                boxShadow: '[4px 4px 0 0 #000]',
              })}
            >
              <div class={css({ maxWidth: '240px', marginX: 'auto' })}>
                <p
                  class={css({
                    fontSize: '13px',
                    lineHeight: '[1.8]',
                    color: 'gray.700',
                    textAlign: 'justify',
                    fontWeight: 'medium',
                  })}
                >
                  적절한 본문 폭은 읽기 경험을 크게 향상시킵니다. 너무 넓으면 시선이 분산되고, 너무 좁으면 답답함을 느낄 수 있습니다.
                </p>
              </div>
            </div>
          </div>
        </div>

        <div class={css({ marginTop: 'auto', paddingTop: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={SettingsIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>양식 설정</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
            본문 폭, 들여쓰기, 문단 간격 등 세밀한 양식 설정으로 읽기 편한 환경을 만들 수 있습니다.
          </p>
        </div>
      </Bento>
    </div>
  </div>
</section>
