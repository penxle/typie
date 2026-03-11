import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';

class Tappable extends HookWidget {
  const Tappable({
    required this.onTap,
    required this.child,
    this.onLongPress,
    this.padding,
    this.debugTapArea = false,
    super.key,
  });

  final Widget child;
  final EdgeInsetsGeometry? padding;
  // ignore: avoid_futureor_void -- to many consumers
  final FutureOr<void> Function() onTap;
  // ignore: avoid_futureor_void -- to many consumers
  final FutureOr<void> Function()? onLongPress;
  final bool debugTapArea;

  static Widget scale({required Widget child, double scale = 0.98}) => _TappableScaleWidget(end: scale, child: child);

  @override
  Widget build(BuildContext context) {
    final controller = useAnimationController(duration: const Duration(milliseconds: 60));
    final progress = useAnimation(
      useMemoized(() => CurvedAnimation(parent: controller, curve: Curves.easeOut), [controller]),
    );

    final Widget content = debugTapArea
        ? Container(color: context.colors.accentDanger, padding: padding, child: child)
        : padding == null
        ? child
        : Padding(padding: padding!, child: child);

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () async {
        unawaited(controller.forward().then((_) => controller.reverse()));
        await Future<void>.delayed(const Duration(milliseconds: 60));
        await onTap();
      },
      onLongPress: onLongPress == null
          ? null
          : () async {
              await onLongPress!();
            },
      child: _TappableProgressData(progress: progress, child: content),
    );
  }
}

class _TappableProgressData extends InheritedWidget {
  const _TappableProgressData({required this.progress, required super.child});

  final double progress;

  static double of(BuildContext context) {
    return context.dependOnInheritedWidgetOfExactType<_TappableProgressData>()?.progress ?? 0.0;
  }

  @override
  bool updateShouldNotify(_TappableProgressData oldWidget) => progress != oldWidget.progress;
}

class _TappableScaleWidget extends StatelessWidget {
  const _TappableScaleWidget({required this.end, required this.child});

  final double end;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final progress = _TappableProgressData.of(context);
    final scale = 1.0 + (end - 1.0) * progress;
    return ScaleTransition(scale: AlwaysStoppedAnimation(scale), child: child);
  }
}
