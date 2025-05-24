import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';

extension BottomSheetExtension on BuildContext {
  Future<T?> showBottomSheet<T extends Object?>({required Widget child}) {
    return router.root.pushWidget(
      child,
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
                  child: ColoredBox(color: AppColors.black.withValues(alpha: 0.5)),
                  onTap: () async {
                    await router.root.maybePop();
                  },
                ),
              ),
            ),
            SafeArea(
              bottom: false,
              child: Align(
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

class BottomSheet extends HookWidget {
  const BottomSheet({required this.child, this.floating = false, this.padding, super.key});

  final Widget child;
  final bool floating;
  final EdgeInsetsGeometry? padding;

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
          child: Container(
            key: sheetKey,
            width: double.infinity,
            decoration: BoxDecoration(
              color: AppColors.white,
              borderRadius: BorderRadius.vertical(
                top: const Radius.circular(16),
                bottom: Radius.circular(floating ? 16 : 0),
              ),
            ),
            margin: floating ? Pad(horizontal: 16, bottom: bottomPadding + 16) : null,
            child: ConstrainedBox(
              constraints: BoxConstraints(maxHeight: maxHeight),
              child: Padding(
                padding: Pad(top: 8, bottom: floating ? 16 : bottomPadding + 16),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  spacing: 16,
                  children: [
                    const SizedBox(
                      width: 60,
                      height: 4,
                      child: DecoratedBox(
                        decoration: BoxDecoration(
                          color: AppColors.gray_200,
                          borderRadius: BorderRadius.all(Radius.circular(999)),
                        ),
                      ),
                    ),
                    if (padding == null) child else Padding(padding: padding!, child: child),
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

class BottomMenu extends StatelessWidget {
  const BottomMenu({required this.items, super.key});

  final List<BottomMenuItem> items;

  @override
  Widget build(BuildContext context) {
    return BottomSheet(child: Column(children: items));
  }
}

class BottomMenuItem extends StatelessWidget {
  const BottomMenuItem({
    required this.icon,
    required this.label,
    required this.onTap,
    this.iconColor = AppColors.gray_700,
    this.labelColor = AppColors.gray_950,
    super.key,
  });

  final IconData icon;
  final String label;

  final Color iconColor;
  final Color labelColor;

  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      padding: const Pad(horizontal: 20, vertical: 12),
      onTap: () {
        context.router.pop();
        onTap();
      },
      child: Row(
        spacing: 12,
        children: [
          Icon(icon, size: 24, color: iconColor),
          Text(
            label,
            style: TextStyle(fontSize: 17, fontWeight: FontWeight.w500, color: labelColor),
          ),
        ],
      ),
    );
  }
}
