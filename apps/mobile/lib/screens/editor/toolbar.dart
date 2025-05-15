import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/values.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/animated_indexed_switcher.dart';
import 'package:typie/widgets/vertical_divider.dart';

class EditorToolbar extends HookWidget {
  const EditorToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);
    final keyboardHeight = useValueListenable(scope.keyboardHeight);
    final isKeyboardVisible = useValueListenable(scope.isKeyboardVisible);
    final selectedTextbarIdx = useValueListenable(scope.selectedTextbarIdx);
    final selectedToolboxIdx = useValueListenable(scope.selectedToolboxIdx);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    if (!isKeyboardVisible && selectedToolboxIdx == -1) {
      return const SizedBox.shrink();
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const _Textbar(),
        Box(
          height: 48,
          decoration: const BoxDecoration(
            color: AppColors.white,
            border: Border(top: BorderSide(color: AppColors.gray_200)),
          ),
          child: Row(
            children: [
              Expanded(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  physics: const AlwaysScrollableScrollPhysics(),
                  padding: const Pad(horizontal: 16),
                  child: Row(
                    spacing: 16,
                    children: [
                      _IconToolbarButton(
                        icon: LucideLightIcons.plus,
                        isActive: selectedToolboxIdx == 1,
                        onTap: () async {
                          if (selectedToolboxIdx == 1) {
                            await webViewController?.emitEvent('focus');
                            scope.selectedToolboxIdx.value = -1;
                          } else {
                            scope.selectedToolboxIdx.value = 1;
                            await webViewController?.emitEvent('blur');
                          }
                        },
                      ),
                      _IconToolbarButton(
                        icon: LucideLightIcons.type_,
                        isActive: selectedTextbarIdx != -1,
                        onTap: () async {
                          scope.selectedTextbarIdx.value = selectedTextbarIdx == -1 ? 0 : -1;
                        },
                      ),
                      _IconToolbarButton(
                        icon: LucideLightIcons.image,
                        onTap: () async {
                          await webViewController?.emitEvent('focus');
                          scope.selectedToolboxIdx.value = -1;
                        },
                      ),
                      _IconToolbarButton(icon: LucideLightIcons.undo, onTap: () {}),
                      _IconToolbarButton(icon: LucideLightIcons.redo, onTap: () {}),
                      _IconToolbarButton(
                        icon: LucideLightIcons.settings,
                        onTap: () async {
                          await webViewController?.emitEvent('focus');
                          scope.selectedToolboxIdx.value = -1;
                        },
                      ),
                    ],
                  ),
                ),
              ),
              const Box.gap(16),
              AnimatedIndexedSwitcher(
                index: selectedTextbarIdx == -1 && selectedToolboxIdx == -1 ? 0 : 1,
                children: [
                  _IconToolbarButton(
                    icon: LucideLightIcons.keyboard_off,
                    onTap: () async {
                      await webViewController?.emitEvent('blur');
                    },
                  ),
                  _IconToolbarButton(
                    icon: LucideLightIcons.circle_x,
                    onTap: () async {
                      await webViewController?.emitEvent('focus');
                      scope.selectedTextbarIdx.value = -1;
                      scope.selectedToolboxIdx.value = -1;
                    },
                  ),
                ],
              ),
              const Box.gap(16),
            ],
          ),
        ),
        Box(
          height: keyboardHeight,
          decoration: const BoxDecoration(
            color: AppColors.white,
            border: Border(top: BorderSide(color: AppColors.gray_200)),
          ),
          child: AnimatedIndexedSwitcher(
            index: max(selectedToolboxIdx, 0),
            children: [
              const SizedBox.expand(),
              GridView.extent(
                maxCrossAxisExtent: 100,
                padding: const Pad(all: 12),
                mainAxisSpacing: 12,
                crossAxisSpacing: 12,
                children: [
                  _BoxButton(
                    icon: LucideLightIcons.image,
                    label: '이미지',
                    isActive: proseMirrorState?.isNodeActive('image') ?? false,
                    onTap: () async {
                      await scope.command('image');
                    },
                  ),
                  _BoxButton(
                    icon: LucideLightIcons.paperclip,
                    label: '파일',
                    isActive: proseMirrorState?.isNodeActive('file') ?? false,
                    onTap: () async {
                      await scope.command('file');
                    },
                  ),
                  _BoxButton(
                    icon: LucideLightIcons.file_up,
                    label: '임베드',
                    isActive: proseMirrorState?.isNodeActive('embed') ?? false,
                    onTap: () async {
                      await scope.command('embed');
                    },
                  ),
                  _BoxButton(
                    icon: TypieIcons.horizontal_rule,
                    label: '구분선',
                    isActive: proseMirrorState?.isNodeActive('horizontal_rule') ?? false,
                    onTap: () async {
                      await scope.command('horizontal_rule');
                    },
                  ),
                  _BoxButton(
                    icon: LucideLightIcons.quote,
                    label: '인용구',
                    isActive: proseMirrorState?.isNodeActive('blockquote') ?? false,
                    onTap: () async {
                      await scope.command('blockquote');
                    },
                  ),
                  _BoxButton(
                    icon: LucideLightIcons.gallery_vertical_end,
                    label: '콜아웃',
                    isActive: proseMirrorState?.isNodeActive('callout') ?? false,
                    onTap: () async {
                      await scope.command('callout');
                    },
                  ),
                  _BoxButton(
                    icon: LucideLightIcons.chevrons_down_up,
                    label: '폴드',
                    isActive: proseMirrorState?.isNodeActive('fold') ?? false,
                    onTap: () async {
                      await scope.command('fold');
                    },
                  ),
                  _BoxButton(
                    icon: LucideLightIcons.table,
                    label: '표',
                    isActive: proseMirrorState?.isNodeActive('table') ?? false,
                    onTap: () async {
                      await scope.command('table');
                    },
                  ),
                  _BoxButton(
                    icon: LucideLightIcons.list,
                    label: '목록',
                    isActive:
                        (proseMirrorState?.isNodeActive('bullet_list') ?? false) ||
                        (proseMirrorState?.isNodeActive('ordered_list') ?? false),
                    onTap: () async {},
                  ),
                  _BoxButton(
                    icon: LucideLightIcons.code,
                    label: '코드',
                    isActive: proseMirrorState?.isNodeActive('code_block') ?? false,
                    onTap: () async {
                      await scope.command('code');
                    },
                  ),
                  _BoxButton(
                    icon: LucideLightIcons.code_xml,
                    label: 'HTML',
                    isActive: proseMirrorState?.isNodeActive('html_block') ?? false,
                    onTap: () async {
                      await scope.command('html');
                    },
                  ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _Textbar extends HookWidget {
  const _Textbar();

  @override
  Widget build(BuildContext context) {
    final controller = useAnimationController(duration: const Duration(milliseconds: 150));
    final curve = useMemoized(
      () => CurvedAnimation(parent: controller, curve: Curves.easeOut, reverseCurve: Curves.easeIn),
      [controller],
    );
    final tweenedOpacity = useMemoized(() => Tween<double>(begin: 0, end: 1).animate(curve), [curve]);
    final tweenedSizeFactor = useMemoized(() => Tween<double>(begin: 0, end: 1).animate(curve), [curve]);

    final scope = EditorStateScope.of(context);
    final selectedTextbarIdx = useValueListenable(scope.selectedTextbarIdx);

    final isVisible = selectedTextbarIdx != -1;
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
        child: Box(
          width: double.infinity,
          height: 48,
          decoration: const BoxDecoration(
            color: AppColors.gray_50,
            border: Border(top: BorderSide(color: AppColors.gray_200)),
          ),
          child: HookBuilder(
            builder: (context) {
              final controller = useAnimationController(duration: const Duration(milliseconds: 150));
              final curve = useMemoized(
                () => CurvedAnimation(parent: controller, curve: Curves.easeOut, reverseCurve: Curves.easeIn),
                [controller],
              );

              final proseMirrorState = useValueListenable(scope.proseMirrorState);

              final defaultOpacityTween = Tween<double>(begin: 1, end: 0);
              final alternateOpacityTween = Tween<double>(begin: 0, end: 1);
              final defaultPositionLeftTween = Tween<double>(begin: 0, end: -10);
              final alternatePositionLeftTween = Tween<double>(begin: 10, end: 0);

              final isAlternate = selectedTextbarIdx > 0;

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
                        child: Opacity(opacity: defaultOpacityTween.evaluate(curve), child: const _DefaultTextbar()),
                      ),
                      if (!controller.isDismissed)
                        Positioned.fill(
                          left: alternatePositionLeftTween.evaluate(curve),
                          child: Opacity(
                            opacity: alternateOpacityTween.evaluate(curve),
                            child: _AlternateTextbar(
                              children: [
                                _SelectTextbar(
                                  name: 'textColor',
                                  activeValue:
                                      proseMirrorState?.getMarkAttributes('text_style')?['textColor'] as String? ??
                                      editorDefaultValues['textColor'],
                                  builder: (context, e, isActive) {
                                    return _ColorToolbarButton(
                                      hex: e['hex'] as String,
                                      isActive: isActive,
                                      onTap: () {
                                        scope.command('text_style', attrs: {'textColor': e['value']});
                                      },
                                    );
                                  },
                                ),
                                _SelectTextbar(
                                  name: 'fontFamily',
                                  activeValue:
                                      proseMirrorState?.getMarkAttributes('text_style')?['fontFamily'] as String? ??
                                      editorDefaultValues['fontFamily'],
                                  builder: (context, e, isActive) {
                                    return _TextToolbarButton(
                                      text: e['label'] as String,
                                      isActive: isActive,
                                      onTap: () {
                                        scope.command('text_style', attrs: {'fontFamily': e['value']});
                                      },
                                    );
                                  },
                                ),
                                _SelectTextbar(
                                  name: 'fontSize',
                                  activeValue:
                                      proseMirrorState?.getMarkAttributes('text_style')?['fontSize'] as num? ??
                                      editorDefaultValues['fontSize'],
                                  builder: (context, e, isActive) {
                                    return _TextToolbarButton(
                                      text: e['label'] as String,
                                      isActive: isActive,
                                      onTap: () {
                                        scope.command('text_style', attrs: {'fontSize': e['value']});
                                      },
                                    );
                                  },
                                ),
                                _SelectTextbar(
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
                                _SelectTextbar(
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
                                _SelectTextbar(
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
                          ),
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

class _DefaultTextbar extends HookWidget {
  const _DefaultTextbar();

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const Pad(horizontal: 16),
      child: Row(
        spacing: 12,
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
            text:
                editorValues['fontFamily']?.firstWhere(
                      (e) =>
                          e['value'] ==
                          (proseMirrorState?.getMarkAttributes('text_style')?['fontFamily'] as String? ??
                              editorDefaultValues['fontFamily']),
                    )['label']
                    as String? ??
                '(알 수 없음)',
            onTap: () {
              scope.selectedTextbarIdx.value = 2;
            },
          ),
          _TextToolbarButton(
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
          const AppVerticalDivider(height: 20, color: AppColors.gray_200),
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
          const AppVerticalDivider(height: 20, color: AppColors.gray_200),
          _IconToolbarButton(icon: LucideLightIcons.link, onTap: () {}),
          _IconToolbarButton(icon: TypieIcons.ruby, onTap: () {}),
          const AppVerticalDivider(height: 20, color: AppColors.gray_200),
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
        const Box.gap(4),
        _IconToolbarButton(
          icon: LucideLightIcons.chevron_left,
          onTap: () {
            scope.selectedTextbarIdx.value = 0;
          },
        ),
        const Box.gap(12),
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

class _SelectTextbar extends HookWidget {
  const _SelectTextbar({required this.name, required this.activeValue, required this.builder});

  final String name;
  final dynamic activeValue;
  // ignore: avoid_positional_boolean_parameters for simplicity
  final Widget Function(BuildContext context, Map<String, dynamic> e, bool isActive) builder;

  @override
  Widget build(BuildContext context) {
    final keys = useMemoized(() => List.generate(editorValues[name]!.length, (_) => GlobalKey()), []);

    useAsyncEffect(() async {
      final index = editorValues[name]!.indexWhere((e) => e['value'] == activeValue);

      if (index != -1 && keys[index].currentContext != null) {
        await Scrollable.ensureVisible(
          keys[index].currentContext!,
          alignment: 0.45,
          duration: const Duration(milliseconds: 150),
        );
      }

      return null;
    });

    return Row(
      spacing: 12,
      children: [
        ...editorValues[name]!.mapIndexed(
          (index, e) => KeyedSubtree(key: keys[index], child: builder(context, e, e['value'] == activeValue)),
        ),
      ],
    );
  }
}

enum _ButtonState { idle, pressed, active }

class _BaseButton extends HookWidget {
  const _BaseButton({required this.onTap, required this.builder, this.isActive = false});

  final Widget Function(BuildContext context, Color color) builder;

  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    final state = useState(_ButtonState.idle);
    final effectiveState =
        state.value == _ButtonState.pressed
            ? _ButtonState.pressed
            : isActive
            ? _ButtonState.active
            : _ButtonState.idle;

    final controller = useAnimationController(duration: const Duration(milliseconds: 150));
    final curve = useMemoized(() => CurvedAnimation(parent: controller, curve: Curves.ease), [controller]);
    final colorTween = useRef<ColorTween?>(null);

    useEffect(() {
      final begin = colorTween.value?.evaluate(curve) ?? (isActive ? AppColors.brand_500 : AppColors.gray_700);

      final end = switch (effectiveState) {
        _ButtonState.idle => AppColors.gray_700,
        _ButtonState.pressed => AppColors.gray_300,
        _ButtonState.active => AppColors.brand_500,
      };

      colorTween.value = ColorTween(begin: begin, end: end);
      controller.forward(from: 0);

      return null;
    }, [effectiveState]);

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      onTapDown: (_) => state.value = _ButtonState.pressed,
      onTapUp: (_) => state.value = _ButtonState.idle,
      onTapCancel: () => state.value = _ButtonState.idle,
      child: AnimatedBuilder(
        animation: controller,
        builder: (context, child) {
          final color = colorTween.value?.evaluate(curve) ?? AppColors.gray_700;

          return builder(context, color);
        },
      ),
    );
  }
}

class _IconToolbarButton extends HookWidget {
  const _IconToolbarButton({required this.onTap, required this.icon, this.isActive = false});

  final IconData icon;

  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return _BaseButton(
      isActive: isActive,
      onTap: onTap,
      builder: (context, color) {
        return Box(padding: const Pad(all: 4), child: Icon(icon, size: 22, color: color));
      },
    );
  }
}

class _ColorToolbarButton extends HookWidget {
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
      child: Container(
        width: 24,
        height: 24,
        decoration: BoxDecoration(
          border: Border.all(
            width: 2,
            color: isActive ? (hex == '#ffffff' ? AppColors.gray_200 : color) : AppColors.transparent,
          ),
          borderRadius: BorderRadius.circular(999),
        ),
        child: Box(
          padding: const Pad(all: 1),
          child: Box(
            decoration: BoxDecoration(
              color: color,
              border: Border.all(color: AppColors.gray_200),
              borderRadius: BorderRadius.circular(999),
            ),
          ),
        ),
      ),
    );
  }
}

class _TextToolbarButton extends HookWidget {
  const _TextToolbarButton({required this.onTap, required this.text, this.isActive = false});

  final String text;
  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return _BaseButton(
      isActive: isActive,
      onTap: onTap,
      builder: (context, color) {
        return Box(
          padding: const Pad(all: 4),
          alignment: Alignment.center,
          child: Text(text, style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: color)),
        );
      },
    );
  }
}

class _BoxButton extends StatelessWidget {
  const _BoxButton({required this.icon, required this.label, required this.onTap, this.isActive = false});

  final IconData icon;
  final String label;
  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return _BaseButton(
      isActive: isActive,
      onTap: onTap,
      builder: (context, color) {
        return Box(
          alignment: Alignment.center,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            spacing: 8,
            children: [
              Icon(icon, size: 24, color: color),
              Text(label, style: TextStyle(fontSize: 12, fontWeight: FontWeight.w500, color: color)),
            ],
          ),
        );
      },
    );
  }
}
