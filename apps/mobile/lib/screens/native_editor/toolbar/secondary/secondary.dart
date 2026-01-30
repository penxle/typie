import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/font_family.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/font_size.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/font_weight.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/letter_spacing.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/line_height.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/text_align.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/text_background_color.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/text_color.dart';

class NativeEditorSecondaryToolbar extends HookWidget {
  const NativeEditorSecondaryToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final controller = useAnimationController(duration: const Duration(milliseconds: 150));
    final curve = useMemoized(
      () => CurvedAnimation(parent: controller, curve: Curves.easeOut, reverseCurve: Curves.easeIn),
      [controller],
    );
    final tweenedOpacity = useMemoized(() => Tween<double>(begin: 0, end: 1).animate(curve), [curve]);
    final tweenedSizeFactor = useMemoized(() => Tween<double>(begin: 0, end: 1).animate(curve), [curve]);

    final scope = NativeEditorToolbarScope.of(context);
    final secondaryToolbarMode = useValueListenable(scope.secondaryToolbarMode);
    final isVisible = secondaryToolbarMode != SecondaryToolbarMode.hidden;

    useEffect(() {
      if (isVisible) {
        unawaited(controller.forward());
      } else {
        unawaited(controller.reverse());
      }

      return null;
    }, [isVisible]);

    return SizeTransition(
      sizeFactor: tweenedSizeFactor,
      axisAlignment: -1,
      child: FadeTransition(
        opacity: tweenedOpacity,
        child: Container(
          width: double.infinity,
          height: 48,
          decoration: BoxDecoration(
            color: context.colors.surfaceDefault,
            border: Border(top: BorderSide(color: context.colors.borderSubtle)),
          ),
          child: HookBuilder(
            builder: (context) {
              final switchController = useAnimationController(duration: const Duration(milliseconds: 150));
              final switchCurve = useMemoized(
                () => CurvedAnimation(parent: switchController, curve: Curves.easeOut, reverseCurve: Curves.easeIn),
                [switchController],
              );

              final textOpacityTween = Tween<double>(begin: 1, end: 0);
              final optionsOpacityTween = Tween<double>(begin: 0, end: 1);
              final textPositionLeftTween = Tween<double>(begin: 0, end: -10);
              final optionsPositionLeftTween = Tween<double>(begin: 10, end: 0);

              final isOptions =
                  secondaryToolbarMode != SecondaryToolbarMode.hidden &&
                  secondaryToolbarMode != SecondaryToolbarMode.text;

              final optionsToolbarMode = useState(secondaryToolbarMode);

              useEffect(() {
                if (isOptions) {
                  unawaited(switchController.forward());
                } else {
                  unawaited(switchController.reverse());
                }

                return null;
              }, [isOptions]);

              useEffect(() {
                if (isOptions) {
                  optionsToolbarMode.value = secondaryToolbarMode;
                }

                return null;
              }, [secondaryToolbarMode, isOptions]);

              return AnimatedBuilder(
                animation: switchController,
                builder: (context, child) {
                  return Stack(
                    alignment: Alignment.centerLeft,
                    children: [
                      Positioned.fill(
                        left: textPositionLeftTween.evaluate(switchCurve),
                        child: Opacity(
                          opacity: textOpacityTween.evaluate(switchCurve),
                          child: const NativeEditorTextToolbar(),
                        ),
                      ),
                      if (!switchController.isDismissed)
                        Positioned.fill(
                          left: optionsPositionLeftTween.evaluate(switchCurve),
                          child: Opacity(
                            opacity: optionsOpacityTween.evaluate(switchCurve),
                            child: DecoratedBox(
                              decoration: BoxDecoration(color: context.colors.surfaceDefault),
                              child: switch (optionsToolbarMode.value) {
                                SecondaryToolbarMode.textColor => const NativeEditorTextColorTextOptionsToolbar(),
                                SecondaryToolbarMode.textBackgroundColor =>
                                  const NativeEditorTextBackgroundColorTextOptionsToolbar(),
                                SecondaryToolbarMode.fontFamily => const NativeEditorFontFamilyTextOptionsToolbar(),
                                SecondaryToolbarMode.fontWeight => const NativeEditorFontWeightTextOptionsToolbar(),
                                SecondaryToolbarMode.fontSize => const NativeEditorFontSizeTextOptionsToolbar(),
                                SecondaryToolbarMode.textAlign => const NativeEditorTextAlignTextOptionsToolbar(),
                                SecondaryToolbarMode.lineHeight => const NativeEditorLineHeightTextOptionsToolbar(),
                                SecondaryToolbarMode.letterSpacing =>
                                  const NativeEditorLetterSpacingTextOptionsToolbar(),
                                _ => const SizedBox.shrink(),
                              },
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
