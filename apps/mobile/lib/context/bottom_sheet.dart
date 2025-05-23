import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/styles/colors.dart';

extension BottomSheetExtension on BuildContext {
  Future<T?> showBottomSheet<T extends Object?>(Widget child) {
    return router.root.pushWidget(
      _Widget(child: child),
      opaque: false,
      transitionBuilder: (context, animation, secondaryAnimation, child) {
        final tweenedBackdropOpacity = Tween<double>(
          begin: 0,
          end: 1,
        ).animate(CurvedAnimation(parent: animation, curve: Curves.easeOutCubic, reverseCurve: Curves.easeIn));

        final tweenedSlide = Tween(
          begin: const Offset(0, 1),
          end: Offset.zero,
        ).animate(CurvedAnimation(parent: animation, curve: Curves.easeOutCubic, reverseCurve: Curves.easeIn));

        return Stack(
          children: [
            Positioned.fill(
              child: FadeTransition(
                opacity: tweenedBackdropOpacity,
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  child: Box(color: AppColors.black.withValues(alpha: 0.5)),
                  onTap: () async {
                    await router.root.maybePop();
                  },
                ),
              ),
            ),
            SafeArea(
              bottom: false,
              child: Box(
                alignment: Alignment.bottomCenter,
                child: SlideTransition(position: tweenedSlide, child: child),
              ),
            ),
          ],
        );
      },
    );
  }
}

class _Widget extends HookWidget {
  const _Widget({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final sheetKey = useMemoized(GlobalKey.new);
    final controller = useAnimationController(upperBound: double.infinity, duration: const Duration(milliseconds: 300));

    final mediaQuery = MediaQuery.of(context);
    final maxHeight = (mediaQuery.size.height - mediaQuery.padding.top) * 0.9;
    final bottomPadding = mediaQuery.padding.bottom;

    return GestureDetector(
      onVerticalDragStart: (details) {
        controller.stop();
      },
      onVerticalDragUpdate: (details) {
        controller.value = controller.value + details.delta.dy;
      },
      onVerticalDragEnd: (details) {
        final size = sheetKey.currentContext?.size;
        if (size == null) {
          return;
        }

        final sheetHeight = size.height;
        final currentOffset = controller.value;
        final velocity = details.primaryVelocity ?? 0.0;

        if (velocity > 300 || (velocity >= -300 && currentOffset > sheetHeight * 0.4)) {
          if (context.mounted) {
            context.router.pop();
          }
        } else {
          controller
            ..duration = const Duration(milliseconds: 300)
            ..animateTo(0, curve: Curves.easeOutCubic);
        }
      },
      child: AnimatedBuilder(
        animation: controller,
        builder: (context, child) {
          return Transform.translate(offset: Offset(0, controller.value), child: child);
        },
        child: Material(
          color: AppColors.transparent,
          child: Box(
            key: sheetKey,
            width: double.infinity,
            decoration: const BoxDecoration(
              color: AppColors.white,
              borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
            ),
            child: ConstrainedBox(
              constraints: BoxConstraints(maxHeight: maxHeight),
              child: Box(
                padding: Pad(horizontal: 24, top: 8, bottom: bottomPadding + 12),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  spacing: 16,
                  children: [
                    const Box(
                      width: 60,
                      height: 4,
                      decoration: BoxDecoration(
                        color: AppColors.gray_200,
                        borderRadius: BorderRadius.all(Radius.circular(999)),
                      ),
                    ),
                    child,
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
