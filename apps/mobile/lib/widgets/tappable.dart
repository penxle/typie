import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';

class Tappable extends StatelessWidget {
  const Tappable({required this.child, required this.onTap, super.key});

  final Widget child;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(behavior: HitTestBehavior.opaque, onTap: onTap, child: child);
  }
}

class AnimatedTappable extends HookWidget {
  const AnimatedTappable({
    required this.builder,
    required this.onTap,
    this.duration = const Duration(milliseconds: 150),
    super.key,
  });

  final Widget Function(BuildContext context, Animation<double> animation) builder;
  final Duration duration;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    final animationController = useAnimationController(duration: duration);

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      onTapDown: (_) => animationController.forward(),
      onTapUp: (_) => animationController.reverse(),
      onTapCancel: animationController.reverse,
      child: builder(context, animationController),
    );
  }
}
