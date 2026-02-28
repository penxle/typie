import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:pointer_interceptor/pointer_interceptor.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/responsive_container.dart';
import 'package:typie/widgets/tappable.dart';

extension BottomSheetExtension on BuildContext {
  Future<T?> showBottomSheet<T extends Object?>({
    required Widget child,
    bool intercept = false,
    double overlayOpacity = 0.5,
    bool resizeToAvoidBottomInset = false,
    bool dismissKeyboardOnTap = true,
    void Function(double)? onHeightCalculated,
  }) {
    unawaited(HapticFeedback.lightImpact());

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
                    child: SizedBox.expand(
                      child: ColoredBox(color: context.colors.overlayDefault.withValues(alpha: overlayOpacity)),
                    ),
                  ),
                  onTap: () async {
                    await context.router.root.maybePop();
                  },
                ),
              ),
            ),
            SafeArea(
              bottom: false,
              child: AnimatedPadding(
                duration: const Duration(milliseconds: 100),
                curve: Curves.easeOut,
                padding: EdgeInsets.only(
                  bottom: resizeToAvoidBottomInset ? MediaQuery.viewInsetsOf(context).bottom : 0,
                ),
                child: Align(
                  alignment: Alignment.bottomCenter,
                  child: ResponsiveContainer(
                    alignment: Alignment.bottomCenter,
                    child: SlideTransition(
                      position: tweenedSlide,
                      child: _BottomSheet(
                        onHeightCalculated: onHeightCalculated,
                        dismissKeyboardOnTap: dismissKeyboardOnTap,
                        child: child,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}

class _BottomSheet extends HookWidget {
  const _BottomSheet({required this.child, this.onHeightCalculated, this.dismissKeyboardOnTap = true});

  final Widget child;
  final void Function(double)? onHeightCalculated;
  final bool dismissKeyboardOnTap;

  @override
  Widget build(BuildContext context) {
    final sheetKey = useMemoized(GlobalKey.new);
    final controller = useAnimationController(upperBound: double.infinity, duration: const Duration(milliseconds: 300));

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        final renderBox = sheetKey.currentContext?.findRenderObject() as RenderBox?;
        if (renderBox != null && onHeightCalculated != null) {
          onHeightCalculated!(renderBox.size.height);
        }
      });
      return null;
    }, []);

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
          controller.duration = const Duration(milliseconds: 300);
          unawaited(controller.animateTo(0, curve: Curves.easeOutCubic));
        }
      },
      child: AnimatedBuilder(
        animation: controller,
        builder: (context, child) {
          return Transform.translate(offset: Offset(0, controller.value), child: child);
        },
        child: _maybeDismissKeyboard(
          dismissKeyboardOnTap: dismissKeyboardOnTap,
          child: Material(
            color: AppColors.transparent,
            child: Container(
              key: sheetKey,
              width: double.infinity,
              decoration: BoxDecoration(
                border: Border(
                  top: BorderSide(color: context.colors.borderStrong),
                  left: BorderSide(color: context.colors.borderStrong),
                  right: BorderSide(color: context.colors.borderStrong),
                ),
                borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
              ),
              child: ClipRRect(
                borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
                child: child,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

Widget _maybeDismissKeyboard({required bool dismissKeyboardOnTap, required Widget child}) {
  return dismissKeyboardOnTap ? KeyboardDismiss(child: child) : child;
}

class AppBottomSheet extends StatelessWidget {
  const AppBottomSheet({required this.child, this.padding, this.includeBottomPadding = true, super.key});

  final Widget child;
  final EdgeInsetsGeometry? padding;
  final bool includeBottomPadding;

  @override
  Widget build(BuildContext context) {
    final mediaQuery = MediaQuery.of(context);
    final maxHeight = (mediaQuery.size.height - mediaQuery.padding.top) * 0.9;
    final bottomPadding = mediaQuery.padding.bottom;

    return Container(
      constraints: BoxConstraints(maxHeight: maxHeight),
      decoration: BoxDecoration(color: context.colors.surfaceSubtle),
      child: Padding(
        padding: Pad(top: 8, bottom: includeBottomPadding ? (bottomPadding + 12) : 0),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          spacing: 16,
          children: [
            SizedBox(
              width: 60,
              height: 4,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: context.colors.borderDefault,
                  borderRadius: const BorderRadius.all(Radius.circular(999)),
                ),
              ),
            ),
            if (padding == null) child else Padding(padding: padding!, child: child),
          ],
        ),
      ),
    );
  }
}

class AppFullBottomSheet extends StatelessWidget {
  const AppFullBottomSheet({required this.title, required this.child, this.padding, super.key});

  final String title;
  final EdgeInsetsGeometry? padding;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final mediaQuery = MediaQuery.of(context);
    final maxHeight = mediaQuery.size.shortestSide > 600
        ? (mediaQuery.size.height - mediaQuery.padding.top) * 0.7
        : double.infinity;
    final bottomPadding = mediaQuery.padding.bottom;

    return ConstrainedBox(
      constraints: BoxConstraints(maxHeight: maxHeight),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Container(
            height: 52,
            decoration: BoxDecoration(
              color: context.colors.surfaceDefault,
              border: Border(bottom: BorderSide(color: context.colors.borderDefault)),
            ),
            padding: const Pad(horizontal: 8),
            child: NavigationToolbar(
              leading: Tappable(
                padding: const Pad(horizontal: 4),
                onTap: () async {
                  await context.router.maybePop();
                },
                child: Icon(LucideLightIcons.x, size: 24, color: context.colors.textDefault),
              ),
              middle: Text(title, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w500)),
            ),
          ),
          Expanded(
            child: Container(
              decoration: BoxDecoration(color: context.colors.surfaceDefault),
              padding: padding ?? Pad(top: 20, bottom: bottomPadding + 12, horizontal: 20),
              child: child,
            ),
          ),
        ],
      ),
    );
  }
}

class BottomMenu extends StatelessWidget {
  const BottomMenu({this.header, required this.items, super.key});

  final Widget? header;
  final List<Widget> items;

  @override
  Widget build(BuildContext context) {
    return AppBottomSheet(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (header != null) ...[
            Padding(padding: const Pad(horizontal: 24), child: header),
            const Gap(16),
            HorizontalDivider(color: context.colors.borderDefault),
            const Gap(16),
          ],
          ...items,
        ],
      ),
    );
  }
}

class BottomMenuSeparator extends StatelessWidget {
  const BottomMenuSeparator({super.key});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const Pad(vertical: 8),
      child: HorizontalDivider(color: context.colors.borderDefault),
    );
  }
}

class BottomMenuItem extends StatelessWidget {
  const BottomMenuItem({
    required this.icon,
    required this.label,
    required this.onTap,
    this.iconColor,
    this.labelColor,
    this.trailing,
    super.key,
  });

  final IconData icon;
  final String label;
  final Widget? trailing;

  final Color? iconColor;
  final Color? labelColor;

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
          Icon(icon, size: 20, color: iconColor ?? context.colors.textDefault),
          Expanded(
            child: Text(label, style: TextStyle(fontSize: 17, color: labelColor ?? context.colors.textDefault)),
          ),
          ?trailing,
        ],
      ),
    );
  }
}

class ConfirmBottomSheet extends StatelessWidget {
  const ConfirmBottomSheet({
    this.title,
    this.message,
    this.child,
    required this.onConfirm,
    this.onCancel,
    this.confirmText = '확인',
    this.cancelText = '취소',
    this.confirmTextColor,
    this.confirmBackgroundColor,
    super.key,
  });

  final String? title;
  final String? message;
  final Widget? child;

  final String confirmText;
  final String cancelText;

  final Color? confirmTextColor;
  final Color? confirmBackgroundColor;

  final void Function() onConfirm;
  final void Function()? onCancel;

  @override
  Widget build(BuildContext context) {
    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (title != null) ...[
            Text(title!, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
            const Gap(8),
          ],
          if (message != null) ...[
            Text(message!, style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
            const Gap(24),
          ],
          if (child != null) ...[child!, const Gap(24)],
          Row(
            spacing: 8,
            children: [
              Expanded(
                child: Tappable(
                  onTap: () async {
                    await context.router.maybePop();
                    onCancel?.call();
                  },
                  child: Container(
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      color: context.colors.surfaceMuted,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    padding: const Pad(vertical: 16),
                    child: Text(cancelText, style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
                  ),
                ),
              ),
              Expanded(
                child: Tappable(
                  onTap: () async {
                    onConfirm();
                    if (context.mounted) {
                      await context.router.maybePop();
                    }
                  },
                  child: Container(
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      color: confirmBackgroundColor ?? context.colors.surfaceInverse,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    padding: const Pad(vertical: 16),
                    child: Text(
                      confirmText,
                      style: TextStyle(
                        fontSize: 16,
                        fontWeight: FontWeight.w600,
                        color: confirmTextColor ?? context.colors.textInverse,
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class AppDraggableBottomSheet extends StatelessWidget {
  const AppDraggableBottomSheet({
    required this.builder,
    super.key,
    this.initialChildSize = 0.4,
    this.minChildSize = 0.25,
    this.maxChildSize = 0.8,
  });

  final Widget Function(BuildContext context, ScrollController scrollController) builder;
  final double initialChildSize;
  final double minChildSize;
  final double maxChildSize;

  @override
  Widget build(BuildContext context) {
    return DraggableScrollableSheet(
      initialChildSize: initialChildSize,
      minChildSize: minChildSize,
      maxChildSize: maxChildSize,
      expand: false,
      builder: (context, scrollController) {
        return Container(
          decoration: BoxDecoration(
            color: context.colors.surfaceSubtle,
            borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
            border: Border(
              top: BorderSide(color: context.colors.borderStrong),
              left: BorderSide(color: context.colors.borderStrong),
              right: BorderSide(color: context.colors.borderStrong),
            ),
          ),
          child: Column(
            children: [
              const SizedBox(height: 8),
              SizedBox(
                width: 60,
                height: 4,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: context.colors.borderDefault,
                    borderRadius: const BorderRadius.all(Radius.circular(999)),
                  ),
                ),
              ),
              const Gap(16),
              Expanded(child: builder(context, scrollController)),
            ],
          ),
        );
      },
    );
  }
}
