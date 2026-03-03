import 'package:flutter/material.dart';

class Fader extends StatelessWidget {
  const Fader({
    required this.show,
    required this.child,
    super.key,
    this.duration = const Duration(milliseconds: 150),
    this.curve = Curves.easeOut,
  });

  final bool show;
  final Widget child;
  final Duration duration;
  final Curve curve;

  @override
  Widget build(BuildContext context) {
    return AnimatedSwitcher(
      duration: duration,
      reverseDuration: duration,
      switchInCurve: curve,
      switchOutCurve: curve,
      transitionBuilder: (child, animation) => FadeTransition(opacity: animation, child: child),
      layoutBuilder: (currentChild, previousChildren) {
        return Stack(children: [...previousChildren, ?currentChild]);
      },
      child: show
          ? KeyedSubtree(key: const ValueKey('fade-visible'), child: child)
          : const SizedBox.shrink(key: ValueKey('fade-hidden')),
    );
  }
}
