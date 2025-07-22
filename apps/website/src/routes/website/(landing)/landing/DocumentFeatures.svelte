<script lang="ts">
  import AnchorIcon from '~icons/lucide/anchor';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import FileIcon from '~icons/lucide/file';
  import FileTextIcon from '~icons/lucide/file-text';
  import FilterIcon from '~icons/lucide/filter';
  import FolderIcon from '~icons/lucide/folder';
  import FolderTreeIcon from '~icons/lucide/folder-tree';
  import PlusIcon from '~icons/lucide/plus';
  import SearchIcon from '~icons/lucide/search';
  import ShapesIcon from '~icons/lucide/shapes';
  import BookmarkFilledIcon from '~icons/typie/bookmark-filled';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import Bento from './Bento.svelte';

  const folderStructure = [
    {
      name: '소설 원고',
      type: 'folder',
      depth: 0,
      open: true,
      children: [
        {
          name: '1부. 시작',
          type: 'folder',
          depth: 1,
          open: true,
          children: [
            { name: '1장. 첫 만남', type: 'file', depth: 2 },
            { name: '2장. 우연한 재회', type: 'file', depth: 2, selected: true },
            { name: '3장. 운명의 장난', type: 'file', depth: 2 },
          ],
        },
        {
          name: '2부. 갈등',
          type: 'folder',
          depth: 1,
          open: false,
          children: [
            { name: '4장. 오해의 시작', type: 'file', depth: 2 },
            { name: '5장. 깊어지는 의심', type: 'file', depth: 2 },
          ],
        },
        { name: '3부. 결말', type: 'folder', depth: 1, open: false },
      ],
    },
    {
      name: '등장인물 설정',
      type: 'folder',
      depth: 0,
      open: false,
      children: [
        { name: '주인공 프로필', type: 'file', depth: 1 },
        { name: '조연 캐릭터', type: 'file', depth: 1 },
      ],
    },
    {
      name: '취재 자료',
      type: 'folder',
      depth: 0,
      open: false,
    },
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
      <Icon icon={FolderTreeIcon} size={16} />
      체계적인 자료 관리
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
      쉽게 정리하고
      <br />
      <span
        class={css({
          backgroundColor: 'amber.400',
          paddingX: '20px',
          display: 'inline-block',
          transform: 'rotate(1deg)',
        })}
      >
        편하게
      </span>
      찾으세요
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
      수많은 글과 자료도 체계적으로 관리하면 찾기 쉽습니다.
      <br />
      다양한 편의 기능으로 생각을 깔끔하게 정리해 보세요.
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
            <Icon icon={FolderTreeIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>폴더</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
            폴더를 만들어 포스트와 자료를 주제별로 깔끔하게 분류하고 관리할 수 있습니다.
          </p>
        </div>

        <div
          class={css({
            display: 'flex',
            height: '350px',
            overflow: 'hidden',
            border: '4px solid',
            borderColor: 'gray.900',
            boxShadow: '[4px 4px 0 0 #000]',
          })}
        >
          <div
            class={css({
              width: '200px',
              borderRightWidth: '4px',
              borderColor: 'gray.900',
              backgroundColor: 'gray.50',
            })}
          >
            <div
              class={css({
                paddingX: '12px',
                paddingY: '10px',
                borderBottomWidth: '4px',
                borderColor: 'gray.900',
                backgroundColor: 'amber.50',
              })}
            >
              <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
                <span class={css({ fontSize: '13px', fontWeight: 'bold', color: 'gray.900', textTransform: 'uppercase' })}>내 포스트</span>
                <Icon style={css.raw({ color: 'gray.900' })} icon={PlusIcon} size={14} />
              </div>
            </div>

            <div class={css({ paddingY: '8px' })}>
              {#each folderStructure as folder (folder.name)}
                <div>
                  <div
                    class={css({
                      display: 'flex',
                      alignItems: 'center',
                      gap: '6px',
                      paddingX: '8px',
                      paddingY: '6px',
                      marginX: '4px',
                      cursor: 'pointer',
                      transition: 'all',
                      transitionDuration: '150ms',
                      fontWeight: 'semibold',
                      _hover: {
                        backgroundColor: 'amber.100',
                      },
                    })}
                  >
                    <Icon style={css.raw({ color: 'gray.700' })} icon={folder.open ? ChevronDownIcon : ChevronRightIcon} size={14} />
                    <Icon style={css.raw({ color: 'amber.600' })} icon={FolderIcon} size={14} />
                    <span class={css({ fontSize: '13px', fontWeight: 'bold', color: 'gray.900' })}>{folder.name}</span>
                  </div>
                  {#if folder.children && folder.open}
                    <div class={css({ marginLeft: '16px', borderLeftWidth: '2px', borderColor: 'gray.300' })}>
                      {#each folder.children as child (child.name)}
                        {#if child.type === 'folder'}
                          <div>
                            <div
                              class={css({
                                display: 'flex',
                                alignItems: 'center',
                                gap: '6px',
                                paddingX: '8px',
                                paddingY: '5px',
                                marginLeft: '12px',
                                marginRight: '4px',
                                cursor: 'pointer',
                                transition: 'all',
                                transitionDuration: '150ms',
                                fontWeight: 'semibold',
                                _hover: {
                                  backgroundColor: 'amber.100',
                                },
                              })}
                            >
                              <div class={css({ size: '4px', backgroundColor: 'gray.400' })}></div>
                              <Icon
                                style={css.raw({ color: 'gray.700' })}
                                icon={'open' in child && child.open ? ChevronDownIcon : ChevronRightIcon}
                                size={14}
                              />
                              <Icon style={css.raw({ color: 'amber.600' })} icon={FolderIcon} size={14} />
                              <span class={css({ fontSize: '13px', fontWeight: 'bold', color: 'gray.900' })}>{child.name}</span>
                            </div>
                            {#if 'children' in child && child.children && 'open' in child && child.open}
                              <div class={css({ marginLeft: '28px', borderLeftWidth: '2px', borderColor: 'gray.300' })}>
                                {#each child.children as grandchild (grandchild.name)}
                                  <div
                                    class={css({
                                      display: 'flex',
                                      alignItems: 'center',
                                      gap: '6px',
                                      paddingX: '8px',
                                      paddingY: '5px',
                                      marginLeft: '12px',
                                      marginRight: '4px',
                                      cursor: 'pointer',
                                      backgroundColor: 'selected' in grandchild && grandchild.selected ? 'amber.100' : 'transparent',
                                      transition: 'all',
                                      transitionDuration: '150ms',
                                      fontWeight: 'semibold',
                                      _hover: {
                                        backgroundColor: 'amber.100',
                                      },
                                    })}
                                  >
                                    <div class={css({ size: '4px', backgroundColor: 'gray.400' })}></div>
                                    <Icon style={css.raw({ color: 'gray.600' })} icon={FileIcon} size={12} />
                                    <span
                                      class={css({
                                        fontSize: '13px',
                                        color: 'selected' in grandchild && grandchild.selected ? 'gray.900' : 'gray.700',
                                        fontWeight: 'selected' in grandchild && grandchild.selected ? 'bold' : 'semibold',
                                      })}
                                    >
                                      {grandchild.name}
                                    </span>
                                  </div>
                                {/each}
                              </div>
                            {/if}
                          </div>
                        {:else}
                          <div
                            class={css({
                              display: 'flex',
                              alignItems: 'center',
                              gap: '6px',
                              paddingX: '8px',
                              paddingY: '5px',
                              marginLeft: '12px',
                              marginRight: '4px',
                              cursor: 'pointer',
                              backgroundColor: 'selected' in child && child.selected ? 'amber.100' : 'transparent',
                              transition: 'all',
                              transitionDuration: '150ms',
                              fontWeight: 'semibold',
                              _hover: {
                                backgroundColor: 'amber.100',
                              },
                            })}
                          >
                            <div class={css({ size: '4px', backgroundColor: 'gray.400' })}></div>
                            <Icon style={css.raw({ color: 'gray.600' })} icon={FileIcon} size={12} />
                            <span
                              class={css({
                                fontSize: '13px',
                                color: 'selected' in child && child.selected ? 'gray.900' : 'gray.700',
                                fontWeight: 'selected' in child && child.selected ? 'bold' : 'semibold',
                              })}
                            >
                              {child.name}
                            </span>
                          </div>
                        {/if}
                      {/each}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>

          <div
            class={css({
              flex: '1',
              display: 'flex',
              flexDirection: 'column',
              backgroundColor: 'white',
            })}
          >
            <div
              class={css({
                paddingX: '20px',
                paddingY: '10px',
                borderBottomWidth: '4px',
                borderColor: 'gray.900',
                backgroundColor: 'gray.50',
              })}
            >
              <div class={flex({ alignItems: 'center', gap: '6px', fontSize: '12px', color: 'gray.600', fontWeight: 'semibold' })}>
                <span>소설 원고</span>
                <Icon icon={ChevronRightIcon} size={12} />
                <span>1부. 시작</span>
                <Icon icon={ChevronRightIcon} size={12} />
                <span class={css({ color: 'gray.900', fontWeight: 'bold' })}>2장. 우연한 재회</span>
              </div>
            </div>

            <div class={css({ flex: '1', padding: '24px', overflow: 'hidden' })}>
              <div class={css({ height: '32px', width: '200px', backgroundColor: 'gray.900', marginBottom: '20px' })}></div>

              <div class={css({ display: 'flex', flexDirection: 'column', gap: '16px' })}>
                <div class={css({ display: 'flex', flexDirection: 'column', gap: '8px' })}>
                  <div class={css({ height: '16px', width: 'full', backgroundColor: 'gray.200' })}></div>
                  <div class={css({ height: '16px', width: '[95%]', backgroundColor: 'gray.200' })}></div>
                  <div class={css({ height: '16px', width: '[85%]', backgroundColor: 'gray.200' })}></div>
                </div>

                <div class={css({ display: 'flex', flexDirection: 'column', gap: '8px' })}>
                  <div class={css({ height: '16px', width: '[90%]', backgroundColor: 'gray.200' })}></div>
                  <div class={css({ height: '16px', width: '[60%]', backgroundColor: 'gray.200' })}></div>
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
            <Icon icon={AnchorIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>앵커</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
            긴 글 안에서 중요한 부분에 앵커를 설정해두면, 스크롤 없이 한 번에 원하는 위치로 이동할 수 있습니다.
          </p>
        </div>

        <div
          class={css({
            overflow: 'hidden',
            border: '4px solid',
            borderColor: 'gray.900',
            boxShadow: '[4px 4px 0 0 #000]',
          })}
        >
          <div
            class={css({
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              paddingX: '16px',
              paddingY: '12px',
              backgroundColor: 'gray.900',
              borderBottomWidth: '4px',
              borderColor: 'gray.900',
            })}
          >
            <div class={flex({ gap: '6px' })}>
              <div class={css({ size: '12px', backgroundColor: '[#FF5F57]' })}></div>
              <div class={css({ size: '12px', backgroundColor: '[#FFBD2E]' })}></div>
              <div class={css({ size: '12px', backgroundColor: '[#28CA42]' })}></div>
            </div>
          </div>

          <div
            class={css({
              display: 'flex',
              gap: '0',
              height: '300px',
              backgroundColor: 'white',
            })}
          >
            <div
              class={css({
                flex: '1',
                padding: '24px',
                paddingRight: '48px',
                overflow: 'hidden',
                position: 'relative',
              })}
            >
              <div class={css({ display: 'flex', flexDirection: 'column', gap: '16px' })}>
                <p
                  class={css({
                    fontSize: '14px',
                    lineHeight: '[1.8]',
                    color: 'gray.700',
                    marginTop: '-40px',
                    fontFamily: 'Pretendard',
                    fontWeight: 'medium',
                  })}
                >
                  ...했다. 그렇게 해서 우리는 처음 만났고, 서로의 이야기를 나누기 시작했다. 그날 밤은 유독 길게 느껴졌다. 창밖으로는
                  끊임없이 비가 내리고 있었고, 우리는 따뜻한 차를 마시며 서로의 이야기에 귀를 기울였다. 벽난로에서 타오르는 장작 소리가
                  고요한 실내에 은은하게 울려 퍼졌다.
                </p>

                <div class={css({ position: 'relative' })}>
                  <p
                    class={css({
                      fontSize: '14px',
                      lineHeight: '[1.8]',
                      color: 'gray.700',
                      fontFamily: 'Pretendard',
                      fontWeight: 'medium',
                    })}
                  >
                    "당신은 어떤 삶을 꿈꾸나요?" 그녀가 물었다. 나는 잠시 망설였다. 이런 질문을 받은 것이 얼마만인지 기억조차 나지 않았다.
                    어린 시절에는 매일같이 꿈을 꾸었지만, 어느새 현실에 치여 꿈조차 잊고 살아가고 있었다.
                  </p>
                  <button
                    class={css({
                      position: 'absolute',
                      top: '0',
                      right: '-24px',
                      padding: '4px',
                      color: 'amber.500',
                      transition: 'all',
                      transitionDuration: '150ms',
                      _hover: {
                        transform: 'scale(1.2) rotate(15deg)',
                      },
                    })}
                    type="button"
                  >
                    <Icon icon={BookmarkFilledIcon} size={18} />
                  </button>
                </div>

                <p
                  class={css({ fontSize: '14px', lineHeight: '[1.8]', color: 'gray.700', fontFamily: 'Pretendard', fontWeight: 'medium' })}
                >
                  "저는요," 나는 천천히 입을 열었다. "평범하지만 의미 있는 하루하루를 살고 싶어요. 매일 아침 일어날 때 오늘 하루가 기대되고,
                  저녁에 잠들 때는 감사함을 느낄 수 있는 그런 삶이요." 내 대답에 그녀는 부드럽게 미소 지었다. "그거야말로 가장 어려운 꿈이
                  아닐까요?"
                </p>
              </div>

              <div
                class={css({
                  position: 'absolute',
                  top: '0',
                  left: '0',
                  right: '0',
                  height: '20px',
                  background:
                    '[linear-gradient(to bottom, rgba(255, 255, 255, 0.8) 0%, rgba(255, 255, 255, 0.4) 50%, rgba(255, 255, 255, 0) 100%)]',
                  pointerEvents: 'none',
                })}
              ></div>

              <div
                class={css({
                  position: 'absolute',
                  bottom: '0',
                  left: '0',
                  right: '0',
                  height: '30px',
                  background:
                    '[linear-gradient(to top, rgba(255, 255, 255, 0.8) 0%, rgba(255, 255, 255, 0.4) 50%, rgba(255, 255, 255, 0) 100%)]',
                  pointerEvents: 'none',
                })}
              ></div>
            </div>

            <div
              class={css({
                width: '20px',
                backgroundColor: 'gray.100',
                borderLeftWidth: '4px',
                borderColor: 'gray.900',
                position: 'relative',
              })}
            >
              <div
                class={css({
                  position: 'absolute',
                  top: '8px',
                  bottom: '8px',
                  left: '[50%]',
                  width: '6px',
                  backgroundColor: 'gray.300',
                  transform: 'translateX(-50%)',
                })}
              ></div>

              <div
                class={css({
                  position: 'absolute',
                  top: '[25%]',
                  left: '[50%]',
                  width: '8px',
                  height: '80px',
                  backgroundColor: 'gray.500',
                  transform: 'translateX(-50%)',
                })}
              ></div>

              <div
                class={css({
                  position: 'absolute',
                  top: '[15%]',
                  right: '0',
                  left: '0',
                  height: '4px',
                  backgroundColor: 'amber.400',
                })}
              ></div>

              <div
                class={css({
                  position: 'absolute',
                  top: '[15%]',
                  right: '[100%]',
                  marginRight: '12px',
                  paddingX: '10px',
                  paddingY: '4px',
                  backgroundColor: 'gray.900',
                  color: 'white',
                  fontSize: '12px',
                  fontWeight: 'bold',
                  whiteSpace: 'nowrap',
                  transform: 'translateY(-50%) rotate(-2deg)',
                  border: '3px solid',
                  borderColor: 'gray.900',
                  boxShadow: '[3px 3px 0 0 #000]',
                  textTransform: 'uppercase',
                })}
              >
                서로의 이야기
              </div>

              <div
                class={css({
                  position: 'absolute',
                  top: '[28%]',
                  right: '0',
                  left: '0',
                  height: '4px',
                  backgroundColor: 'amber.400',
                })}
              ></div>

              <div
                class={css({
                  position: 'absolute',
                  top: '[28%]',
                  right: '[100%]',
                  marginRight: '12px',
                  paddingX: '10px',
                  paddingY: '4px',
                  backgroundColor: 'gray.900',
                  color: 'white',
                  fontSize: '12px',
                  fontWeight: 'bold',
                  whiteSpace: 'nowrap',
                  transform: 'translateY(-50%) rotate(2deg)',
                  border: '3px solid',
                  borderColor: 'gray.900',
                  boxShadow: '[3px 3px 0 0 #000]',
                  textTransform: 'uppercase',
                })}
              >
                삶의 질문
              </div>

              <div
                class={css({
                  position: 'absolute',
                  top: '[78%]',
                  right: '0',
                  left: '0',
                  height: '4px',
                  backgroundColor: 'amber.400',
                })}
              ></div>

              <div
                class={css({
                  position: 'absolute',
                  top: '[78%]',
                  right: '[100%]',
                  marginRight: '12px',
                  paddingX: '10px',
                  paddingY: '4px',
                  backgroundColor: 'gray.900',
                  color: 'white',
                  fontSize: '12px',
                  fontWeight: 'bold',
                  whiteSpace: 'nowrap',
                  transform: 'translateY(-50%) rotate(-1deg)',
                  border: '3px solid',
                  borderColor: 'gray.900',
                  boxShadow: '[3px 3px 0 0 #000]',
                  textTransform: 'uppercase',
                })}
              >
                마음의 고백
              </div>
            </div>
          </div>
        </div>
      </Bento>
    </div>
  </div>

  <div class={flex({ gap: '24px' })}>
    <div class={css({ flex: '1' })}>
      <Bento>
        <div class={css({ marginBottom: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
            <Icon icon={SearchIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>검색</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
            포스트 제목, 폴더명, 본문 내용까지 한 번에 검색하여 원하는 정보를 빠르게 찾을 수 있습니다.
          </p>
        </div>

        <div class={center({ flexGrow: 1 })}>
          <div class={css({ width: 'full', maxWidth: '400px' })}>
            <div
              class={css({
                display: 'flex',
                alignItems: 'center',
                gap: '12px',
                padding: '12px',
                backgroundColor: 'white',
                border: '4px solid',
                borderColor: 'gray.900',
                boxShadow: '[4px 4px 0 0 #000]',
                marginBottom: '16px',
              })}
            >
              <Icon style={css.raw({ color: 'gray.700' })} icon={SearchIcon} size={20} />
              <input
                class={css({
                  flex: '1',
                  fontSize: '14px',
                  color: 'gray.900',
                  outline: 'none',
                  fontWeight: 'semibold',
                  '&::placeholder': {
                    color: 'gray.500',
                  },
                })}
                placeholder="포스트, 폴더, 내용 검색..."
                readOnly
                type="text"
                value="갈등"
              />
              <button
                class={css({
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                  paddingX: '10px',
                  paddingY: '6px',
                  backgroundColor: 'amber.400',
                  fontSize: '12px',
                  color: 'gray.900',
                  fontWeight: 'bold',
                  textTransform: 'uppercase',
                  border: '3px solid',
                  borderColor: 'gray.900',
                  transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                  cursor: 'pointer',
                  _hover: {
                    transform: 'translateY(-2px)',
                    boxShadow: '[2px 2px 0 0 #000]',
                  },
                })}
                type="button"
              >
                <Icon icon={FilterIcon} size={14} />
                <span>필터</span>
              </button>
            </div>

            <div
              class={css({
                overflow: 'hidden',
                backgroundColor: 'white',
                border: '4px solid',
                borderColor: 'gray.900',
                boxShadow: '[4px 4px 0 0 #000]',
              })}
            >
              <div
                class={css({
                  padding: '12px',
                  borderBottomWidth: '4px',
                  borderColor: 'gray.900',
                  cursor: 'pointer',
                  transition: 'all',
                  transitionDuration: '150ms',
                  _hover: {
                    backgroundColor: 'amber.50',
                  },
                })}
              >
                <div class={flex({ alignItems: 'center', gap: '8px', marginBottom: '4px' })}>
                  <Icon style={css.raw({ color: 'amber.600' })} icon={FolderIcon} size={16} />
                  <span class={css({ fontSize: '13px', fontWeight: 'bold', color: 'gray.900' })}>
                    2부. <mark class={css({ backgroundColor: 'amber.300', paddingX: '2px', fontWeight: 'black' })}>갈등</mark>
                  </span>
                </div>
                <p class={css({ fontSize: '12px', color: 'gray.600', marginLeft: '24px', fontWeight: 'semibold' })}>
                  소설 원고 / 2부. 갈등
                </p>
              </div>

              <div
                class={css({
                  padding: '12px',
                  borderBottomWidth: '4px',
                  borderColor: 'gray.900',
                  cursor: 'pointer',
                  transition: 'all',
                  transitionDuration: '150ms',
                  _hover: {
                    backgroundColor: 'amber.50',
                  },
                })}
              >
                <div class={flex({ alignItems: 'center', gap: '8px', marginBottom: '4px' })}>
                  <Icon style={css.raw({ color: 'gray.700' })} icon={FileTextIcon} size={16} />
                  <span class={css({ fontSize: '13px', fontWeight: 'bold', color: 'gray.900' })}>
                    2장. <mark class={css({ backgroundColor: 'amber.300', paddingX: '2px', fontWeight: 'black' })}>갈등</mark>
                    의 시작
                  </span>
                </div>
                <p class={css({ fontSize: '12px', color: 'gray.700', marginLeft: '24px', lineHeight: '[1.5]', fontWeight: 'medium' })}>
                  ...그들 사이에 작은 <mark class={css({ backgroundColor: 'amber.300', paddingX: '2px', fontWeight: 'black' })}>갈등</mark>
                  이 시작되었다. 처음에는 사소한 의견 차이였지만...
                </p>
              </div>

              <div
                class={css({
                  padding: '12px',
                  cursor: 'pointer',
                  transition: 'all',
                  transitionDuration: '150ms',
                  _hover: {
                    backgroundColor: 'amber.50',
                  },
                })}
              >
                <div class={flex({ alignItems: 'center', gap: '8px', marginBottom: '4px' })}>
                  <Icon style={css.raw({ color: 'gray.700' })} icon={FileTextIcon} size={16} />
                  <span class={css({ fontSize: '13px', fontWeight: 'bold', color: 'gray.900' })}>주인공 프로필</span>
                </div>
                <p class={css({ fontSize: '12px', color: 'gray.700', marginLeft: '24px', lineHeight: '[1.5]', fontWeight: 'medium' })}>
                  ...내면의 <mark class={css({ backgroundColor: 'amber.300', paddingX: '2px', fontWeight: 'black' })}>갈등</mark>
                  을 자주 겪는 성격으로, 결정을 내리는 데 시간이...
                </p>
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
            <Icon icon={ShapesIcon} size={20} />
            <h3 class={css({ fontSize: '20px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>템플릿</h3>
          </div>
          <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.6]', fontWeight: 'medium' })}>
            자주 사용하는 글의 형식을 템플릿으로 만들어두면, 반복을 줄이고 더 빨리 글쓰기를 시작할 수 있습니다.
          </p>
        </div>

        <div class={center({ flexGrow: 1 })}>
          <div class={css({ width: 'full', maxWidth: '480px' })}>
            <div class={flex({ flexDirection: 'column', gap: '8px' })}>
              <button
                class={css({
                  display: 'flex',
                  alignItems: 'center',
                  gap: '12px',
                  paddingX: '16px',
                  paddingY: '12px',
                  backgroundColor: 'white',
                  border: '4px solid',
                  borderColor: 'gray.900',
                  cursor: 'pointer',
                  transition: 'all',
                  transitionDuration: '150ms',
                  textAlign: 'left',
                  width: 'full',
                  boxShadow: '[4px 4px 0 0 #000]',
                  _hover: {
                    transform: 'translate(-2px, -2px)',
                    boxShadow: '[6px 6px 0 0 #000]',
                    backgroundColor: 'amber.50',
                  },
                })}
                type="button"
              >
                <div class={center({ size: '32px', backgroundColor: 'amber.400', border: '3px solid', borderColor: 'gray.900' })}>
                  <Icon style={css.raw({ color: 'gray.900' })} icon={FileTextIcon} size={18} />
                </div>
                <div class={flex({ flexDirection: 'column', gap: '2px' })}>
                  <p class={css({ fontSize: '14px', fontWeight: 'black', color: 'gray.900', textTransform: 'uppercase' })}>일기</p>
                  <p class={css({ fontSize: '12px', color: 'gray.600', fontWeight: 'semibold' })}>날짜, 날씨, 오늘의 일정, 하루 기록</p>
                </div>
              </button>

              <button
                class={css({
                  display: 'flex',
                  alignItems: 'center',
                  gap: '12px',
                  paddingX: '16px',
                  paddingY: '12px',
                  backgroundColor: 'white',
                  border: '4px solid',
                  borderColor: 'gray.900',
                  cursor: 'pointer',
                  transition: 'all',
                  transitionDuration: '150ms',
                  textAlign: 'left',
                  width: 'full',
                  boxShadow: '[4px 4px 0 0 #000]',
                  _hover: {
                    transform: 'translate(-2px, -2px)',
                    boxShadow: '[6px 6px 0 0 #000]',
                    backgroundColor: 'blue.50',
                  },
                })}
                type="button"
              >
                <div class={center({ size: '32px', backgroundColor: 'blue.400', border: '3px solid', borderColor: 'gray.900' })}>
                  <Icon style={css.raw({ color: 'white' })} icon={FileTextIcon} size={18} />
                </div>
                <div class={flex({ flexDirection: 'column', gap: '2px' })}>
                  <p class={css({ fontSize: '14px', fontWeight: 'black', color: 'gray.900', textTransform: 'uppercase' })}>독후감</p>
                  <p class={css({ fontSize: '12px', color: 'gray.600', fontWeight: 'semibold' })}>
                    책 정보, 줄거리, 인상 깊은 구절, 느낀 점
                  </p>
                </div>
              </button>

              <button
                class={css({
                  display: 'flex',
                  alignItems: 'center',
                  gap: '12px',
                  paddingX: '16px',
                  paddingY: '12px',
                  backgroundColor: 'white',
                  border: '4px solid',
                  borderColor: 'gray.900',
                  cursor: 'pointer',
                  transition: 'all',
                  transitionDuration: '150ms',
                  textAlign: 'left',
                  width: 'full',
                  boxShadow: '[4px 4px 0 0 #000]',
                  _hover: {
                    transform: 'translate(-2px, -2px)',
                    boxShadow: '[6px 6px 0 0 #000]',
                    backgroundColor: 'green.50',
                  },
                })}
                type="button"
              >
                <div class={center({ size: '32px', backgroundColor: 'green.400', border: '3px solid', borderColor: 'gray.900' })}>
                  <Icon style={css.raw({ color: 'white' })} icon={FileTextIcon} size={18} />
                </div>
                <div class={flex({ flexDirection: 'column', gap: '2px' })}>
                  <p class={css({ fontSize: '14px', fontWeight: 'black', color: 'gray.900', textTransform: 'uppercase' })}>에세이</p>
                  <p class={css({ fontSize: '12px', color: 'gray.600', fontWeight: 'semibold' })}>제목, 서론, 본론, 결론</p>
                </div>
              </button>

              <button
                class={css({
                  display: 'flex',
                  alignItems: 'center',
                  gap: '12px',
                  paddingX: '16px',
                  paddingY: '12px',
                  marginTop: '8px',
                  backgroundColor: 'gray.50',
                  border: '4px dashed',
                  borderColor: 'gray.400',
                  cursor: 'pointer',
                  transition: 'all',
                  transitionDuration: '150ms',
                  textAlign: 'left',
                  width: 'full',
                  _hover: {
                    backgroundColor: 'gray.100',
                    borderColor: 'gray.900',
                    borderStyle: 'solid',
                  },
                })}
                type="button"
              >
                <div class={center({ size: '32px' })}>
                  <Icon style={css.raw({ color: 'gray.600' })} icon={PlusIcon} size={18} />
                </div>
                <div class={flex({ flexDirection: 'column', gap: '2px' })}>
                  <p class={css({ fontSize: '14px', fontWeight: 'black', color: 'gray.800', textTransform: 'uppercase' })}>
                    내 템플릿 만들기
                  </p>
                  <p class={css({ fontSize: '12px', color: 'gray.600', fontWeight: 'semibold' })}>자주 쓰는 글 구조를 저장</p>
                </div>
              </button>
            </div>
          </div>
        </div>
      </Bento>
    </div>
  </div>
</section>
