import 'package:flutter/material.dart';

class AnimatedIndexedSwitcher extends StatelessWidget {
  const AnimatedIndexedSwitcher({
    required this.children,
    this.index = 0,
    super.key,
    this.fit = StackFit.passthrough,
    this.alignment = Alignment.center,
    this.transitionBuilder = AnimatedSwitcher.defaultTransitionBuilder,
    this.duration = const Duration(milliseconds: 150),
    this.reverseDuration,
    this.switchInCurve = Curves.easeInOut,
    this.switchOutCurve = Curves.easeInOut,
  });

  final int index;
  final List<Widget> children;
  final StackFit fit;
  final AlignmentGeometry alignment;
  final AnimatedSwitcherTransitionBuilder transitionBuilder;
  final Duration duration;
  final Duration? reverseDuration;
  final Curve switchInCurve;
  final Curve switchOutCurve;

  @override
  Widget build(BuildContext context) {
    return AnimatedSwitcher(
      duration: duration,
      reverseDuration: reverseDuration,
      switchInCurve: switchInCurve,
      switchOutCurve: switchOutCurve,
      transitionBuilder: transitionBuilder,
      layoutBuilder: (currentChild, previousChildren) {
        return Stack(
          fit: fit,
          alignment: alignment,
          children: <Widget>[...previousChildren, if (currentChild != null) currentChild],
        );
      },
      child: IndexedStack(key: ValueKey(index), sizing: fit, alignment: alignment, index: index, children: children),
    );
  }
}
