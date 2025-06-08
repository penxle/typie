import 'dart:async';
import 'dart:io';
import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:collection/collection.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/screens/editor/__generated__/persist_blob_as_file.req.gql.dart';
import 'package:typie/screens/editor/__generated__/persist_blob_as_image.req.gql.dart';
import 'package:typie/screens/editor/__generated__/toolbar_site_fragment.data.gql.dart';
import 'package:typie/screens/editor/__generated__/unfurl_embed.req.gql.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/values.dart';
import 'package:typie/services/blob.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/animated_indexed_switcher.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/svg_image.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:typie/widgets/vertical_divider.dart';

class EditorToolbar extends HookWidget {
  const EditorToolbar({required this.site, super.key});

  final GEditorScreen_Toolbar_site site;

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);
    final keyboardHeight = useValueListenable(scope.keyboardHeight);
    final isKeyboardVisible = useValueListenable(scope.isKeyboardVisible);
    final selectedTextbarIdx = useValueListenable(scope.selectedTextbarIdx);
    final selectedToolboxIdx = useValueListenable(scope.selectedToolboxIdx);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    useAsyncEffect(() async {
      if (proseMirrorState?.currentNode != null) {
        scope.selectedTextbarIdx.value = -1;
      }
      return null;
    }, [proseMirrorState?.currentNode]);

    useAsyncEffect(() async {
      if (proseMirrorState?.isNodeActive('blockquote') ?? false) {
        scope.selectedTextbarIdx.value = 0;
      }
      return null;
    }, [proseMirrorState?.isNodeActive('blockquote')]);

    useAsyncEffect(() async {
      if ((proseMirrorState?.isNodeActive('bullet_list') ?? false) ||
          (proseMirrorState?.isNodeActive('ordered_list') ?? false)) {
        scope.selectedTextbarIdx.value = 0;
      }
      return null;
    }, [proseMirrorState?.isNodeActive('bullet_list'), proseMirrorState?.isNodeActive('ordered_list')]);

    if (!isKeyboardVisible && selectedToolboxIdx == -1) {
      return const SizedBox.shrink();
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _SubToolBar(
          isVisible: selectedTextbarIdx != -1,
          isAlternate: selectedTextbarIdx > 0,
          alternate: _AlternateTextbar(
            children: [
              _SelectValuesBar(
                name: 'textColor',
                activeValue:
                    proseMirrorState?.getMarkAttributes('text_style')?['textColor'] as String? ??
                    editorDefaultValues['textColor'],
                builder: (context, e, isActive) {
                  return _ColorToolbarButton(
                    hex: e['hex'] as String,
                    isActive: isActive,
                    onTap: () async {
                      await scope.command('text_style', attrs: {'textColor': e['value']});
                    },
                  );
                },
              ),
              _SelectFontFamilyBar(
                site: site,
                activeValue:
                    proseMirrorState?.getMarkAttributes('text_style')?['fontFamily'] as String? ??
                    editorDefaultValues['fontFamily'],
              ),
              _SelectValuesBar(
                name: 'fontSize',
                activeValue:
                    proseMirrorState?.getMarkAttributes('text_style')?['fontSize'] as num? ??
                    editorDefaultValues['fontSize'],
                builder: (context, e, isActive) {
                  return _TextToolbarButton(
                    text: e['label'] as String,
                    isActive: isActive,
                    onTap: () async {
                      await scope.command('text_style', attrs: {'fontSize': e['value']});
                    },
                  );
                },
              ),
              _SelectValuesBar(
                name: 'textAlign',
                activeValue:
                    proseMirrorState?.getNodeAttributes('paragraph')?['textAlign'] as String? ??
                    editorDefaultValues['textAlign'],
                builder: (context, e, isActive) {
                  return _TextToolbarButton(
                    text: e['label'] as String,
                    isActive: isActive,
                    onTap: () async {
                      await scope.command('paragraph', attrs: {'textAlign': e['value']});
                    },
                  );
                },
              ),
              _SelectValuesBar(
                name: 'lineHeight',
                activeValue:
                    proseMirrorState?.getNodeAttributes('paragraph')?['lineHeight'] as num? ??
                    editorDefaultValues['lineHeight'],
                builder: (context, e, isActive) {
                  return _TextToolbarButton(
                    text: e['label'] as String,
                    isActive: isActive,
                    onTap: () async {
                      await scope.command('paragraph', attrs: {'lineHeight': e['value']});
                    },
                  );
                },
              ),
              _SelectValuesBar(
                name: 'letterSpacing',
                activeValue:
                    proseMirrorState?.getNodeAttributes('paragraph')?['letterSpacing'] as num? ??
                    editorDefaultValues['letterSpacing'],
                builder: (context, e, isActive) {
                  return _TextToolbarButton(
                    text: e['label'] as String,
                    isActive: isActive,
                    onTap: () async {
                      await scope.command('paragraph', attrs: {'letterSpacing': e['value']});
                    },
                  );
                },
              ),
            ],
          ),
          child: _DefaultTextbar(site: site),
        ),
        _SubToolBar(
          isVisible:
              proseMirrorState?.currentNode?.type == 'embed' && proseMirrorState?.currentNode?.attrs?['id'] == null,
          child: const _DefaultEmbedbar(),
        ),
        Container(
          height: 48,
          decoration: const BoxDecoration(
            color: AppColors.white,
            border: Border(top: BorderSide(color: AppColors.gray_100)),
          ),
          padding: const Pad(right: 8),
          child: Row(
            spacing: 8,
            children: [
              Expanded(
                child: () {
                  if (proseMirrorState?.isNodeActive('blockquote') ?? false) {
                    return const _BlockQuoteToolbar();
                  } else if ((proseMirrorState?.isNodeActive('bullet_list') ?? false) ||
                      (proseMirrorState?.isNodeActive('ordered_list') ?? false)) {
                    return const _ListToolbar();
                  } else if (proseMirrorState?.isMarkActive('link') ?? false) {
                    return _NodeToolbar(
                      label: '링크',
                      withDelete: false,
                      children: [Text((proseMirrorState?.getMarkAttributes('link')?['href'] ?? '') as String)],
                    );
                  }

                  switch (proseMirrorState?.currentNode?.type) {
                    case 'file':
                      return const _FileToolbar();
                    case 'image':
                      return const _ImageToolbar();
                    case 'embed':
                      return const _NodeToolbar(label: '임베드', children: []);
                    case 'horizontal_rule':
                      return _NodeToolbar(
                        label: '구분선',
                        children: [
                          _TextToolbarButton(
                            text: '변경',
                            color: AppColors.gray_700,
                            onTap: () async {
                              if (scope.selectedToolboxIdx.value == 2) {
                                await webViewController?.requestFocus();
                              } else {
                                scope.selectedToolboxIdx.value = 2;
                                await webViewController?.clearFocus();
                              }
                            },
                          ),
                        ],
                      );
                    default:
                      return const _DefaultToolbar();
                  }
                }(),
              ),
              _IconToolbarButton(
                icon: LucideLightIcons.chevron_left,
                isRepeatable: true,
                onTap: () async {
                  await webViewController?.requestFocus();
                  await scope.webViewController.value?.emitEvent('caret', {'direction': -1});
                },
              ),
              _IconToolbarButton(
                icon: LucideLightIcons.chevron_right,
                isRepeatable: true,
                onTap: () async {
                  await webViewController?.requestFocus();
                  await scope.webViewController.value?.emitEvent('caret', {'direction': 1});
                },
              ),
              AnimatedIndexedSwitcher(
                index: selectedTextbarIdx == -1 && selectedToolboxIdx == -1 ? 0 : 1,
                children: [
                  _IconToolbarButton(
                    icon: LucideLightIcons.keyboard_off,
                    onTap: () async {
                      await webViewController?.clearFocus();
                    },
                  ),
                  _IconToolbarButton(
                    icon: LucideLightIcons.circle_x,
                    onTap: () async {
                      await webViewController?.requestFocus();
                      scope.selectedTextbarIdx.value = -1;
                    },
                  ),
                ],
              ),
            ],
          ),
        ),
        Container(
          height: keyboardHeight,
          decoration: const BoxDecoration(
            color: AppColors.white,
            border: Border(top: BorderSide(color: AppColors.gray_100)),
          ),
          child: AnimatedIndexedSwitcher(
            index: max(selectedToolboxIdx, 0),
            children: [
              const SizedBox.expand(),
              GridView.extent(
                maxCrossAxisExtent: 96,
                padding: const Pad(all: 16),
                mainAxisSpacing: 16,
                crossAxisSpacing: 16,
                children: [
                  _BoxButton(
                    icon: 'image',
                    label: '이미지',
                    isActive: proseMirrorState?.isNodeActive('image') ?? false,
                    onTap: () async {
                      await scope.command('image');
                      await webViewController?.requestFocus();
                    },
                  ),
                  _BoxButton(
                    icon: 'paperclip',
                    label: '파일',
                    isActive: proseMirrorState?.isNodeActive('file') ?? false,
                    onTap: () async {
                      await scope.command('file');
                      await webViewController?.requestFocus();
                    },
                  ),
                  _BoxButton(
                    icon: 'file-up',
                    label: '임베드',
                    isActive: proseMirrorState?.isNodeActive('embed') ?? false,
                    onTap: () async {
                      await scope.command('embed');
                      await webViewController?.requestFocus();
                    },
                  ),
                  _BoxButton(
                    icon: 'horizontal-rule',
                    label: '구분선',
                    isActive: proseMirrorState?.isNodeActive('horizontal_rule') ?? false,
                    onTap: () async {
                      await scope.command(
                        'horizontal_rule',
                        attrs: {'horizontalRule': editorDefaultValues['horizontalRule']},
                      );
                      await webViewController?.requestFocus();
                    },
                  ),
                  _BoxButton(
                    icon: 'quote',
                    label: '인용구',
                    isActive: proseMirrorState?.isNodeActive('blockquote') ?? false,
                    onTap: () async {
                      await scope.command('blockquote', attrs: {'blockquote': editorDefaultValues['blockquote']});
                      await webViewController?.requestFocus();
                    },
                  ),
                  _BoxButton(
                    icon: 'gallery-vertical-end',
                    label: '콜아웃',
                    isActive: proseMirrorState?.isNodeActive('callout') ?? false,
                    onTap: () async {
                      await scope.command('callout');
                      await webViewController?.requestFocus();
                    },
                  ),
                  _BoxButton(
                    icon: 'chevrons-down-up',
                    label: '폴드',
                    isActive: proseMirrorState?.isNodeActive('fold') ?? false,
                    onTap: () async {
                      await scope.command('fold');
                      await webViewController?.requestFocus();
                    },
                  ),
                  _BoxButton(
                    icon: 'table',
                    label: '표',
                    isActive: proseMirrorState?.isNodeActive('table') ?? false,
                    onTap: () async {
                      await scope.command('table');
                      await webViewController?.requestFocus();
                    },
                  ),
                  _BoxButton(
                    icon: 'list',
                    label: '목록',
                    isActive:
                        (proseMirrorState?.isNodeActive('bullet_list') ?? false) ||
                        (proseMirrorState?.isNodeActive('ordered_list') ?? false),
                    onTap: () async {
                      await scope.command('bullet_list');
                      await webViewController?.requestFocus();
                    },
                  ),
                  _BoxButton(
                    icon: 'code',
                    label: '코드',
                    isActive: proseMirrorState?.isNodeActive('code_block') ?? false,
                    onTap: () async {
                      await scope.command('code_block');
                      await webViewController?.requestFocus();
                    },
                  ),
                  _BoxButton(
                    icon: 'code-xml',
                    label: 'HTML',
                    isActive: proseMirrorState?.isNodeActive('html_block') ?? false,
                    onTap: () async {
                      await scope.command('html_block');
                      await webViewController?.requestFocus();
                    },
                  ),
                ],
              ),
              SingleChildScrollView(
                physics: const AlwaysScrollableScrollPhysics(),
                padding: const Pad(all: 20),
                child: _SelectValuesBar(
                  name: 'horizontalRule',
                  activeValue:
                      proseMirrorState?.getNodeAttributes('horizontal_rule')?['type'] as String? ??
                      editorDefaultValues['horizontalRule'],
                  valueKey: 'type',
                  direction: Axis.vertical,
                  builder: (context, e, isActive) {
                    return _ListButton(
                      component: e['component'] as Widget,
                      isActive: isActive,
                      onTap: () async {
                        await scope.command('horizontal_rule', attrs: {'horizontalRule': e['type']});
                      },
                    );
                  },
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _DefaultToolbar extends HookWidget {
  const _DefaultToolbar();

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);
    final selectedToolboxIdx = useValueListenable(scope.selectedToolboxIdx);
    final selectedTextbarIdx = useValueListenable(scope.selectedTextbarIdx);

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const Pad(left: 8),
      child: Row(
        spacing: 4,
        children: [
          _IconToolbarButton(
            icon: LucideLightIcons.plus,
            isActive: selectedToolboxIdx == 1,
            onTap: () async {
              if (selectedToolboxIdx == 1) {
                await webViewController?.requestFocus();
              } else {
                scope.selectedToolboxIdx.value = 1;
                await webViewController?.clearFocus();
              }
            },
          ),
          _IconToolbarButton(
            icon: LucideLightIcons.type_,
            isActive: selectedTextbarIdx != -1,
            onTap: () {
              scope.selectedTextbarIdx.value = selectedTextbarIdx == -1 ? 0 : -1;
            },
          ),
          _IconToolbarButton(
            icon: LucideLightIcons.image,
            onTap: () async {
              await scope.command('image');
              await webViewController?.requestFocus();
            },
          ),
          _IconToolbarButton(
            icon: LucideLightIcons.undo,
            onTap: () async {
              await scope.command('undo');
            },
          ),
          _IconToolbarButton(
            icon: LucideLightIcons.redo,
            onTap: () async {
              await scope.command('redo');
            },
          ),
        ],
      ),
    );
  }
}

class _NodeToolbar extends HookWidget {
  const _NodeToolbar({this.label, required this.children, this.withDelete = true});

  final String? label;
  final List<Widget> children;
  final bool withDelete;

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const Pad(left: 16),
      child: Row(
        spacing: 8,
        children: [
          if (label != null) ...[
            Text(label!, style: const TextStyle(fontSize: 16, color: AppColors.gray_700)),
            const Gap(0),
            const AppVerticalDivider(height: 20),
            const Gap(0),
          ],
          ...children,
          if (withDelete)
            _TextToolbarButton(
              text: '삭제',
              color: AppColors.red_500,
              onTap: () async {
                await scope.command('delete');
                await webViewController?.requestFocus();
              },
            ),
        ],
      ),
    );
  }
}

class _ImageToolbar extends HookWidget {
  const _ImageToolbar();

  @override
  Widget build(BuildContext context) {
    final blob = useService<Blob>();
    final client = useService<GraphQLClient>();

    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return _NodeToolbar(
      label: '이미지',
      children: [
        if (proseMirrorState!.currentNode!.attrs?['id'] == null)
          _TextToolbarButton(
            text: '업로드',
            color: AppColors.gray_700,
            onTap: () async {
              final nodeId = proseMirrorState.currentNode!.attrs?['nodeId'] as String?;
              if (nodeId == null) {
                return;
              }

              final result = await FilePicker.platform.pickFiles(type: FileType.image);
              if (result == null) {
                return;
              }

              final pickedFile = result.files.firstOrNull;
              if (pickedFile == null) {
                return;
              }

              final file = File(pickedFile.path!);
              final mimetype = await blob.mime(file);

              final url = file.uri.replace(scheme: 'picker', queryParameters: {'type': mimetype}).toString();

              await scope.webViewController.value?.emitEvent('nodeview', {
                'nodeId': nodeId,
                'name': 'inflight',
                'detail': {'url': url},
              });

              try {
                final path = await blob.upload(file);
                final result = await client.request(
                  GEditorScreen_PersistBlobAsImage_MutationReq((b) => b..vars.input.path = path),
                );

                await scope.webViewController.value?.emitEvent('nodeview', {
                  'nodeId': nodeId,
                  'name': 'success',
                  'detail': {
                    'attrs': {
                      'id': result.persistBlobAsImage.id,
                      'url': result.persistBlobAsImage.url,
                      'ratio': result.persistBlobAsImage.ratio,
                      'placeholder': result.persistBlobAsImage.placeholder,
                      'size': result.persistBlobAsImage.size,
                    },
                  },
                });
              } catch (_) {
                await scope.webViewController.value?.emitEvent('nodeview', {'nodeId': nodeId, 'name': 'error'});
              }
            },
          ),
      ],
    );
  }
}

class _ListToolbar extends HookWidget {
  const _ListToolbar();

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return _NodeToolbar(
      withDelete: false,
      children: [
        _IconToolbarButton(
          icon: LucideLightIcons.list,
          isActive: proseMirrorState?.isNodeActive('bullet_list') ?? false,
          onTap: () async {
            await scope.command('bullet_list');
          },
        ),
        _IconToolbarButton(
          icon: LucideLightIcons.list_ordered,
          isActive: proseMirrorState?.isNodeActive('ordered_list') ?? false,
          onTap: () async {
            await scope.command('ordered_list');
          },
        ),
      ],
    );
  }
}

class _BlockQuoteToolbar extends HookWidget {
  const _BlockQuoteToolbar();

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return _NodeToolbar(
      withDelete: false,
      children: [
        Padding(
          padding: const Pad(vertical: 8),
          child: _SelectValuesBar(
            name: 'blockquote',
            activeValue:
                proseMirrorState?.getNodeAttributes('blockquote')?['type'] as String? ??
                editorDefaultValues['blockquote'],
            valueKey: 'type',
            builder: (context, e, isActive) {
              return Center(
                child: _WidgetToolbarButton(
                  widget: e['component'] as Widget,
                  isActive: isActive,
                  onTap: () async {
                    await scope.command('blockquote', attrs: {'blockquote': e['type']});
                  },
                ),
              );
            },
          ),
        ),
      ],
    );
  }
}

class _FileToolbar extends HookWidget {
  const _FileToolbar();

  @override
  Widget build(BuildContext context) {
    final blob = useService<Blob>();
    final client = useService<GraphQLClient>();

    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return _NodeToolbar(
      label: '파일',
      children: [
        if (proseMirrorState!.currentNode!.attrs?['id'] == null)
          _TextToolbarButton(
            text: '업로드',
            color: AppColors.gray_700,
            onTap: () async {
              final nodeId = proseMirrorState.currentNode!.attrs?['nodeId'] as String?;
              if (nodeId == null) {
                return;
              }

              final result = await FilePicker.platform.pickFiles();
              if (result == null) {
                return;
              }

              final pickedFile = result.files.firstOrNull;
              if (pickedFile == null) {
                return;
              }

              final file = File(pickedFile.path!);

              await scope.webViewController.value?.emitEvent('nodeview', {
                'nodeId': nodeId,
                'name': 'inflight',
                'detail': {'name': pickedFile.name, 'size': pickedFile.size},
              });

              try {
                final path = await blob.upload(file);
                final result = await client.request(
                  GEditorScreen_PersistBlobAsFile_MutationReq((b) => b..vars.input.path = path),
                );

                await scope.webViewController.value?.emitEvent('nodeview', {
                  'nodeId': nodeId,
                  'name': 'success',
                  'detail': {
                    'attrs': {
                      'id': result.persistBlobAsFile.id,
                      'url': result.persistBlobAsFile.url,
                      'name': result.persistBlobAsFile.name,
                      'size': result.persistBlobAsFile.size,
                    },
                  },
                });
              } catch (_) {
                await scope.webViewController.value?.emitEvent('nodeview', {'nodeId': nodeId, 'name': 'error'});
              }
            },
          ),
      ],
    );
  }
}

class _SubToolBar extends HookWidget {
  const _SubToolBar({required this.child, required this.isVisible, this.alternate, this.isAlternate = false});

  final Widget child;
  final bool isVisible;
  final Widget? alternate;
  final bool isAlternate;

  @override
  Widget build(BuildContext context) {
    final controller = useAnimationController(duration: const Duration(milliseconds: 150));
    final curve = useMemoized(
      () => CurvedAnimation(parent: controller, curve: Curves.easeOut, reverseCurve: Curves.easeIn),
      [controller],
    );
    final tweenedOpacity = useMemoized(() => Tween<double>(begin: 0, end: 1).animate(curve), [curve]);
    final tweenedSizeFactor = useMemoized(() => Tween<double>(begin: 0, end: 1).animate(curve), [curve]);

    final isDismissed = useState(controller.isDismissed);

    useAsyncEffect(() async {
      if (isVisible) {
        isDismissed.value = false;
        await controller.forward();
      } else {
        await controller.reverse();
        isDismissed.value = true;
      }

      return null;
    }, [isVisible]);

    if (isDismissed.value) {
      return const SizedBox.shrink();
    }

    return SizeTransition(
      sizeFactor: tweenedSizeFactor,
      axisAlignment: -1,
      child: FadeTransition(
        opacity: tweenedOpacity,
        child: Container(
          width: double.infinity,
          height: 48,
          decoration: const BoxDecoration(
            color: AppColors.white,
            border: Border(top: BorderSide(color: AppColors.gray_100)),
          ),
          child: HookBuilder(
            builder: (context) {
              final controller = useAnimationController(duration: const Duration(milliseconds: 150));
              final curve = useMemoized(
                () => CurvedAnimation(parent: controller, curve: Curves.easeOut, reverseCurve: Curves.easeIn),
                [controller],
              );

              final defaultOpacityTween = Tween<double>(begin: 1, end: 0);
              final alternateOpacityTween = Tween<double>(begin: 0, end: 1);
              final defaultPositionLeftTween = Tween<double>(begin: 0, end: -10);
              final alternatePositionLeftTween = Tween<double>(begin: 10, end: 0);

              useEffect(() {
                if (isAlternate) {
                  controller.forward();
                } else {
                  controller.reverse();
                }

                return null;
              }, [isAlternate]);

              return AnimatedBuilder(
                animation: controller,
                builder: (context, child) {
                  return Stack(
                    alignment: Alignment.centerLeft,
                    children: [
                      Positioned.fill(
                        left: defaultPositionLeftTween.evaluate(curve),
                        child: Opacity(opacity: defaultOpacityTween.evaluate(curve), child: this.child),
                      ),
                      if (!controller.isDismissed)
                        Positioned.fill(
                          left: alternatePositionLeftTween.evaluate(curve),
                          child: Opacity(opacity: alternateOpacityTween.evaluate(curve), child: alternate),
                        ),
                    ],
                  );
                },
              );
            },
          ),
        ),
      ),
    );
  }
}

class _DefaultEmbedbar extends HookWidget {
  const _DefaultEmbedbar();

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);
    final client = useService<GraphQLClient>();

    final embedUrl = useState('');

    return Padding(
      padding: const Pad(horizontal: 20),
      child: Row(
        spacing: 8,
        children: [
          Expanded(
            child: TextField(
              autofocus: true,
              smartDashesType: SmartDashesType.disabled,
              smartQuotesType: SmartQuotesType.disabled,
              decoration: const InputDecoration.collapsed(
                hintText: 'https://...',
                hintStyle: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: AppColors.gray_300),
              ),
              onChanged: (value) {
                embedUrl.value = value;
              },
            ),
          ),
          Tappable(
            onTap: () async {
              final nodeId = proseMirrorState?.currentNode!.attrs?['nodeId'] as String?;
              if (nodeId == null) {
                return;
              }

              final url = RegExp(r'^[^:]+:\/\/').hasMatch(embedUrl.value)
                  ? embedUrl.value
                  : 'https://${embedUrl.value}';

              await scope.webViewController.value?.emitEvent('nodeview', {
                'nodeId': nodeId,
                'name': 'inflight',
                'detail': {'url': url},
              });

              try {
                final result = await client.request(
                  GEditorScreen_UnfurlEmbed_MutationReq((b) => b..vars.input.url = url),
                );

                await scope.webViewController.value?.emitEvent('nodeview', {
                  'nodeId': nodeId,
                  'name': 'success',
                  'detail': {
                    'attrs': {
                      'id': result.unfurlEmbed.id,
                      'url': result.unfurlEmbed.url,
                      'title': result.unfurlEmbed.title,
                      'description': result.unfurlEmbed.description,
                      'thumbnailUrl': result.unfurlEmbed.thumbnailUrl,
                      'html': result.unfurlEmbed.html,
                    },
                  },
                });
              } catch (_) {
                await scope.webViewController.value?.emitEvent('nodeview', {'nodeId': nodeId, 'name': 'error'});
              }
            },
            child: const Text('확인'),
          ),
        ],
      ),
    );
  }
}

class _DefaultTextbar extends HookWidget {
  const _DefaultTextbar({required this.site});

  final GEditorScreen_Toolbar_site site;

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const Pad(horizontal: 16),
      child: Row(
        spacing: 4,
        children: [
          _ColorToolbarButton(
            hex:
                editorValues['textColor']?.firstWhere(
                      (e) =>
                          e['value'] ==
                          (proseMirrorState?.getMarkAttributes('text_style')?['textColor'] as String? ??
                              editorDefaultValues['textColor']),
                    )['hex']
                    as String,
            onTap: () {
              scope.selectedTextbarIdx.value = 1;
            },
          ),
          _TextToolbarButton(
            color: AppColors.gray_700,
            text:
                editorValues['fontFamily']?.firstWhereOrNull(
                      (e) =>
                          e['value'] ==
                          (proseMirrorState?.getMarkAttributes('text_style')?['fontFamily'] as String? ??
                              editorDefaultValues['fontFamily']),
                    )?['label']
                    as String? ??
                site.fonts
                    .firstWhereOrNull(
                      (e) => e.id == proseMirrorState?.getMarkAttributes('text_style')?['fontFamily'] as String?,
                    )
                    ?.name ??
                '(알 수 없음)',
            onTap: () {
              scope.selectedTextbarIdx.value = 2;
            },
          ),
          _TextToolbarButton(
            color: AppColors.gray_700,
            text:
                editorValues['fontSize']?.firstWhere(
                      (e) =>
                          e['value'] ==
                          (proseMirrorState?.getMarkAttributes('text_style')?['fontSize'] as num? ??
                              editorDefaultValues['fontSize']),
                    )['label']
                    as String,
            onTap: () {
              scope.selectedTextbarIdx.value = 3;
            },
          ),
          const AppVerticalDivider(height: 20),
          _IconToolbarButton(
            icon: LucideLightIcons.bold,
            isActive: proseMirrorState?.isMarkActive('bold') ?? false,
            onTap: () async {
              await scope.command('bold');
            },
          ),
          _IconToolbarButton(
            icon: LucideLightIcons.italic,
            isActive: proseMirrorState?.isMarkActive('italic') ?? false,
            onTap: () async {
              await scope.command('italic');
            },
          ),
          _IconToolbarButton(
            icon: LucideLightIcons.underline,
            isActive: proseMirrorState?.isMarkActive('underline') ?? false,
            onTap: () async {
              await scope.command('underline');
            },
          ),
          _IconToolbarButton(
            icon: LucideLightIcons.strikethrough,
            isActive: proseMirrorState?.isMarkActive('strike') ?? false,
            onTap: () async {
              await scope.command('strike');
            },
          ),
          const AppVerticalDivider(height: 20),
          _IconToolbarButton(
            icon: LucideLightIcons.link,
            onTap: () async {
              await context.showModal(
                intercept: true,
                child: HookForm(
                  onSubmit: (form) async {
                    await scope.command('link', attrs: {'url': form.data['url']});
                  },
                  builder: (context, form) {
                    return ConfirmModal(
                      title: '링크 삽입',
                      confirmText: '삽입',
                      onConfirm: () async {
                        await form.submit();
                      },
                      child: HookFormTextField.collapsed(
                        initialValue: (proseMirrorState?.getMarkAttributes('link')?['href'] ?? '') as String,
                        name: 'url',
                        placeholder: 'https://...',
                        style: const TextStyle(fontSize: 16),
                        autofocus: true,
                      ),
                    );
                  },
                ),
              );
            },
          ),
          _IconToolbarButton(icon: TypieIcons.ruby, onTap: () {}),
          const AppVerticalDivider(height: 20),
          _IconToolbarButton(
            icon: LucideLightIcons.align_left,
            onTap: () {
              scope.selectedTextbarIdx.value = 4;
            },
          ),
          _IconToolbarButton(
            icon: TypieIcons.line_height,
            onTap: () {
              scope.selectedTextbarIdx.value = 5;
            },
          ),
          _IconToolbarButton(
            icon: TypieIcons.letter_spacing,
            onTap: () {
              scope.selectedTextbarIdx.value = 6;
            },
          ),
        ],
      ),
    );
  }
}

class _AlternateTextbar extends HookWidget {
  const _AlternateTextbar({required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final selectedTextbarIdx = useValueListenable(scope.selectedTextbarIdx);

    return Row(
      children: [
        const Gap(4),
        _IconToolbarButton(
          icon: LucideLightIcons.chevron_left,
          onTap: () {
            scope.selectedTextbarIdx.value = 0;
          },
        ),
        const Gap(12),
        Expanded(
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            physics: const AlwaysScrollableScrollPhysics(),
            padding: const Pad(right: 16),
            child: selectedTextbarIdx > 0 ? children[selectedTextbarIdx - 1] : const SizedBox.shrink(),
          ),
        ),
      ],
    );
  }
}

class _SelectValuesBar extends HookWidget {
  const _SelectValuesBar({
    required this.name,
    required this.activeValue,
    required this.builder,
    this.direction = Axis.horizontal,
    this.valueKey = 'value',
  });

  final String name;
  final dynamic activeValue;
  final Widget Function(BuildContext context, Map<String, dynamic> e, bool isActive) builder;
  final Axis direction;
  final String? valueKey;

  @override
  Widget build(BuildContext context) {
    final keys = useMemoized(() => List.generate(editorValues[name]!.length, (_) => GlobalKey()), []);

    useAsyncEffect(() async {
      final index = editorValues[name]!.indexWhere((e) => e[valueKey] == activeValue);

      if (index != -1 && keys[index].currentContext != null) {
        await Scrollable.ensureVisible(
          keys[index].currentContext!,
          alignment: 0.45,
          duration: const Duration(milliseconds: 150),
        );
      }

      return null;
    }, [activeValue]);

    return Flex(
      direction: direction,
      spacing: 4,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ...editorValues[name]!.mapIndexed(
          (index, e) => KeyedSubtree(key: keys[index], child: builder(context, e, e[valueKey] == activeValue)),
        ),
      ],
    );
  }
}

class _SelectFontFamilyBar extends HookWidget {
  const _SelectFontFamilyBar({required this.site, required this.activeValue});

  final GEditorScreen_Toolbar_site site;
  final dynamic activeValue;

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final count = editorValues['fontFamily']!.length + site.fonts.length;
    final keys = useMemoized(() => List.generate(count, (_) => GlobalKey()), []);

    useAsyncEffect(() async {
      final index1 = editorValues['fontFamily']!.indexWhere((e) => e['value'] == activeValue);
      final index2 = site.fonts.indexWhere((e) => e.id == activeValue);
      final index = index1 != -1
          ? index1
          : index2 != -1
          ? index2 + editorValues['fontFamily']!.length
          : -1;

      if (index != -1 && keys[index].currentContext != null) {
        await Scrollable.ensureVisible(
          keys[index].currentContext!,
          alignment: 0.45,
          duration: const Duration(milliseconds: 150),
        );
      }

      return null;
    }, [activeValue]);

    return Row(
      spacing: 4,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ...editorValues['fontFamily']!.mapIndexed(
          (index, e) => KeyedSubtree(
            key: keys[index],
            child: _TextToolbarButton(
              text: e['label'] as String,
              isActive: e['value'] == activeValue,
              onTap: () async {
                await scope.command('text_style', attrs: {'fontFamily': e['value']});
              },
            ),
          ),
        ),
        ...site.fonts.mapIndexed(
          (index, e) => KeyedSubtree(
            key: keys[index + editorValues['fontFamily']!.length],
            child: _TextToolbarButton(
              text: e.name,
              isActive: e.id == activeValue,
              onTap: () async {
                await scope.command('text_style', attrs: {'fontFamily': e.id});
              },
            ),
          ),
        ),
      ],
    );
  }
}

enum _ButtonState { idle, pressed, active }

class _BaseButton extends HookWidget {
  const _BaseButton({
    required this.onTap,
    required this.builder,
    this.isActive = false,
    this.isRepeatable = false,
    this.color = AppColors.gray_700,
  });

  final Widget Function(BuildContext context, Color color, Color? backgroundColor) builder;

  final Color color;
  final bool isActive;
  final bool isRepeatable;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    final state = useState(_ButtonState.idle);
    final effectiveState = state.value == _ButtonState.pressed
        ? _ButtonState.pressed
        : isActive
        ? _ButtonState.active
        : _ButtonState.idle;

    final controller = useAnimationController(duration: const Duration(milliseconds: 150));
    final curve = useMemoized(() => CurvedAnimation(parent: controller, curve: Curves.ease), [controller]);

    final defaultForegroundColor = isActive ? AppColors.gray_950 : color;
    final foregroundTween = useRef<ColorTween?>(null);
    final backgroundTween = useRef<ColorTween?>(null);

    final repeatTimer = useRef<Timer?>(null);

    useEffect(() {
      foregroundTween.value = ColorTween(
        begin: foregroundTween.value?.evaluate(curve) ?? defaultForegroundColor,
        end: switch (effectiveState) {
          _ButtonState.idle => color,
          _ButtonState.pressed => AppColors.gray_300,
          _ButtonState.active => AppColors.gray_950,
        },
      );

      backgroundTween.value = ColorTween(
        begin: backgroundTween.value?.evaluate(curve),
        end: switch (effectiveState) {
          _ButtonState.idle => null,
          _ButtonState.pressed => null,
          _ButtonState.active => AppColors.gray_100,
        },
      );

      controller.forward(from: 0);

      return null;
    }, [effectiveState]);

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      onLongPressStart: (_) {
        state.value = _ButtonState.pressed;
        if (isRepeatable) {
          repeatTimer.value = Timer.periodic(const Duration(milliseconds: 100), (_) {
            onTap();
          });
        }
      },
      onLongPressEnd: (_) {
        repeatTimer.value?.cancel();
        state.value = _ButtonState.idle;
      },
      onTapDown: (_) => state.value = _ButtonState.pressed,
      onTapUp: (_) => state.value = _ButtonState.idle,
      onTapCancel: () => state.value = _ButtonState.idle,
      child: AnimatedBuilder(
        animation: controller,
        builder: (context, child) {
          final foregroundColor = foregroundTween.value?.evaluate(curve) ?? defaultForegroundColor;
          final backgroundColor = backgroundTween.value?.evaluate(curve);

          return builder(context, foregroundColor, backgroundColor);
        },
      ),
    );
  }
}

class _WidgetToolbarButton extends StatelessWidget {
  const _WidgetToolbarButton({required this.onTap, required this.widget, this.isActive = false});

  final Widget widget;

  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return _BaseButton(
      isActive: isActive,
      onTap: onTap,
      builder: (context, color, backgroundColor) {
        return Container(
          decoration: BoxDecoration(color: backgroundColor, borderRadius: BorderRadius.circular(6)),
          padding: const Pad(all: 8),
          child: widget,
        );
      },
    );
  }
}

class _IconToolbarButton extends StatelessWidget {
  const _IconToolbarButton({required this.onTap, required this.icon, this.isActive = false, this.isRepeatable = false});

  final IconData icon;

  final bool isActive;
  final bool isRepeatable;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return _BaseButton(
      isActive: isActive,
      isRepeatable: isRepeatable,
      onTap: onTap,
      builder: (context, color, backgroundColor) {
        return Container(
          decoration: BoxDecoration(color: backgroundColor, borderRadius: BorderRadius.circular(6)),
          padding: const Pad(all: 8),
          child: Icon(icon, size: 20, color: color),
        );
      },
    );
  }
}

class _ColorToolbarButton extends StatelessWidget {
  const _ColorToolbarButton({required this.onTap, required this.hex, this.isActive = false});

  final String hex;
  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    final color = Color(int.parse('0xFF${hex.substring(1)}'));

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      child: Center(
        child: Container(
          width: 26,
          height: 26,
          decoration: BoxDecoration(
            border: Border.all(
              width: 2,
              color: isActive ? (hex == '#ffffff' ? AppColors.gray_200 : color) : AppColors.transparent,
            ),
            borderRadius: BorderRadius.circular(999),
          ),
          child: Container(
            margin: const Pad(all: 2),
            decoration: BoxDecoration(
              color: color,
              border: Border.all(color: hex == '#ffffff' ? AppColors.gray_200 : color),
              borderRadius: BorderRadius.circular(999),
            ),
          ),
        ),
      ),
    );
  }
}

class _TextToolbarButton extends StatelessWidget {
  const _TextToolbarButton({
    required this.onTap,
    required this.text,
    this.isActive = false,
    this.color = AppColors.gray_400,
  });

  final String text;
  final Color color;
  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return _BaseButton(
      isActive: isActive,
      onTap: onTap,
      color: color,
      builder: (context, color, _) {
        return Center(
          child: Container(
            padding: const Pad(all: 8),
            child: Text(text, style: TextStyle(fontSize: 16, color: color)),
          ),
        );
      },
    );
  }
}

class _BoxButton extends StatelessWidget {
  const _BoxButton({required this.icon, required this.label, required this.onTap, this.isActive = false});

  final String icon;
  final String label;
  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return _BaseButton(
      isActive: isActive,
      onTap: onTap,
      builder: (context, color, backgroundColor) {
        return Column(
          mainAxisAlignment: MainAxisAlignment.center,
          spacing: 12,
          children: [
            SvgImage('icons/$icon', width: 28, height: 28, color: color),
            Text(label, style: TextStyle(fontSize: 15, color: color)),
          ],
        );
      },
    );
  }
}

class _ListButton extends StatelessWidget {
  const _ListButton({required this.component, required this.onTap, this.isActive = false});

  final Widget component;
  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return _BaseButton(
      isActive: isActive,
      onTap: onTap,
      builder: (context, color, backgroundColor) {
        return Container(
          decoration: BoxDecoration(color: backgroundColor, borderRadius: BorderRadius.circular(6)),
          height: 48,
          child: Align(child: component),
        );
      },
    );
  }
}
