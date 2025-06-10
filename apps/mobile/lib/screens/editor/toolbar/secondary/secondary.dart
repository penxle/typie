import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/secondary/text.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/font_family.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/font_size.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/letter_spacing.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/line_height.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/text_align.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/text_color.dart';
import 'package:typie/styles/colors.dart';

class SecondaryToolbar extends HookWidget {
  const SecondaryToolbar({super.key});

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
    final secondaryToolbarMode = useValueListenable(scope.secondaryToolbarMode);
    final isVisible = secondaryToolbarMode != SecondaryToolbarMode.hidden;

    useEffect(() {
      if (isVisible) {
        controller.forward();
      } else {
        controller.reverse();
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
          decoration: const BoxDecoration(
            color: AppColors.white,
            border: Border(top: BorderSide(color: AppColors.gray_100)),
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

              useEffect(() {
                if (isOptions) {
                  switchController.forward();
                } else {
                  switchController.reverse();
                }

                return null;
              }, [isOptions]);

              return AnimatedBuilder(
                animation: switchController,
                builder: (context, child) {
                  return Stack(
                    alignment: Alignment.centerLeft,
                    children: [
                      Positioned.fill(
                        left: textPositionLeftTween.evaluate(switchCurve),
                        child: Opacity(opacity: textOpacityTween.evaluate(switchCurve), child: const TextToolbar()),
                      ),
                      if (!switchController.isDismissed)
                        Positioned.fill(
                          left: optionsPositionLeftTween.evaluate(switchCurve),
                          child: Opacity(
                            opacity: optionsOpacityTween.evaluate(switchCurve),
                            child: switch (secondaryToolbarMode) {
                              SecondaryToolbarMode.textColor => const TextColorTextOptionsToolbar(),
                              SecondaryToolbarMode.fontFamily => const FontFamilyTextOptionsToolbar(),
                              SecondaryToolbarMode.fontSize => const FontSizeTextOptionsToolbar(),
                              SecondaryToolbarMode.textAlign => const TextAlignTextOptionsToolbar(),
                              SecondaryToolbarMode.lineHeight => const LineHeightTextOptionsToolbar(),
                              SecondaryToolbarMode.letterSpacing => const LetterSpacingTextOptionsToolbar(),
                              _ => const SizedBox.shrink(),
                            },
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
