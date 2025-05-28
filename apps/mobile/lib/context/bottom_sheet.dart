import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:pointer_interceptor/pointer_interceptor.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';

extension BottomSheetExtension on BuildContext {
  Future<T?> showBottomSheet<T extends Object?>({required Widget child, bool intercept = false}) {
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
              child: PointerInterceptor(
                intercepting: intercept,
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  child: FadeTransition(
                    opacity: tweenedBackdropOpacity,
                    child: SizedBox.expand(child: ColoredBox(color: AppColors.black.withValues(alpha: 0.5))),
                  ),
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

class _BottomSheet extends HookWidget {
  const _BottomSheet({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final sheetKey = useMemoized(GlobalKey.new);
    final controller = useAnimationController(upperBound: double.infinity, duration: const Duration(milliseconds: 300));

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
            decoration: const BoxDecoration(
              border: Border(top: BorderSide(color: AppColors.gray_950)),
              borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
            ),
            child: ClipRRect(
              borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
              child: child,
            ),
          ),
        ),
      ),
    );
  }
}

class AppBottomSheet extends StatelessWidget {
  const AppBottomSheet({required this.child, this.padding, super.key});

  final Widget child;
  final EdgeInsetsGeometry? padding;

  @override
  Widget build(BuildContext context) {
    final mediaQuery = MediaQuery.of(context);
    final maxHeight = (mediaQuery.size.height - mediaQuery.padding.top) * 0.9;
    final bottomPadding = mediaQuery.padding.bottom;

    return _BottomSheet(
      child: Container(
        constraints: BoxConstraints(maxHeight: maxHeight),
        decoration: const BoxDecoration(color: AppColors.gray_50),
        child: Padding(
          padding: Pad(top: 8, bottom: bottomPadding + 12),
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
    );
  }
}

class AppFullBottomSheet extends StatelessWidget {
  const AppFullBottomSheet({required this.title, required this.child, this.padding = const Pad(all: 20), super.key});

  final String title;
  final EdgeInsetsGeometry? padding;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return _BottomSheet(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Container(
            height: 52,
            decoration: const BoxDecoration(
              color: AppColors.white,
              border: Border(bottom: BorderSide(color: AppColors.gray_200)),
            ),
            padding: const Pad(horizontal: 8),
            child: NavigationToolbar(
              leading: Tappable(
                padding: const Pad(horizontal: 4),
                onTap: () async {
                  await context.router.maybePop();
                },
                child: const Icon(LucideLightIcons.x, size: 24, color: AppColors.gray_950),
              ),
              middle: Text(title, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w500)),
            ),
          ),
          Expanded(
            child: Container(
              decoration: const BoxDecoration(color: AppColors.white),
              padding: padding,
              child: child,
            ),
          ),
        ],
      ),
    );
  }
}

class BottomMenu extends StatelessWidget {
  const BottomMenu({required this.items, super.key});

  final List<BottomMenuItem> items;

  @override
  Widget build(BuildContext context) {
    return AppBottomSheet(child: Column(children: items));
  }
}

class BottomMenuItem extends StatelessWidget {
  const BottomMenuItem({
    required this.icon,
    required this.label,
    required this.onTap,
    this.iconColor = AppColors.gray_950,
    this.labelColor = AppColors.gray_950,
    this.trailing,
    super.key,
  });

  final IconData icon;
  final String label;
  final Widget? trailing;

  final Color iconColor;
  final Color labelColor;

  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      padding: const Pad(horizontal: 24, vertical: 10),
      onTap: () {
        context.router.pop();
        onTap();
      },
      child: Row(
        spacing: 16,
        children: [
          Icon(icon, size: 20, color: iconColor),
          Expanded(
            child: Text(label, style: TextStyle(fontSize: 17, color: labelColor)),
          ),
          if (trailing != null) trailing!,
        ],
      ),
    );
  }
}
