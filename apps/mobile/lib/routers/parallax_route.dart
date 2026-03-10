import 'dart:async';

import 'package:flutter/cupertino.dart';
import 'package:flutter/gestures.dart';

const _kCurve = Cubic(0.25, 0.46, 0.1, 1);
const _kCornerRadius = 40.0;
const _kBackGestureWidth = 20.0;
const _kMinFlingVelocity = 1.0;

class ParallaxPageRoute<T> extends PageRoute<T> {
  ParallaxPageRoute({required this.content, super.settings, super.fullscreenDialog});

  final Widget content;

  @override
  Color? get barrierColor => null;

  @override
  String? get barrierLabel => null;

  @override
  bool get maintainState => true;

  @override
  Duration get transitionDuration => const Duration(milliseconds: 450);

  @override
  Duration get reverseTransitionDuration => const Duration(milliseconds: 450);

  @override
  bool canTransitionTo(TransitionRoute<dynamic> nextRoute) {
    if (nextRoute is PageRoute && nextRoute.fullscreenDialog) {
      return false;
    }

    if (!nextRoute.opaque) {
      return false;
    }

    return true;
  }

  bool get _canPopWithGesture {
    if (!navigator!.canPop()) {
      return false;
    }

    if (isFirst || willHandlePopInternally || fullscreenDialog) {
      return false;
    }

    if (animation!.status != AnimationStatus.completed) {
      return false;
    }

    if (secondaryAnimation!.status != AnimationStatus.dismissed) {
      return false;
    }

    if (navigator!.userGestureInProgress) {
      return false;
    }

    return true;
  }

  @override
  Widget buildPage(BuildContext context, Animation<double> animation, Animation<double> secondaryAnimation) {
    return content;
  }

  @override
  Widget buildTransitions(
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) {
    if (fullscreenDialog) {
      return CupertinoFullscreenDialogTransition(
        primaryRouteAnimation: animation,
        secondaryRouteAnimation: secondaryAnimation,
        linearTransition: navigator!.userGestureInProgress,
        child: child,
      );
    }

    final linear = navigator!.userGestureInProgress;

    final primaryPosition = Tween<Offset>(
      begin: const Offset(1, 0),
      end: Offset.zero,
    ).animate(linear ? animation : CurvedAnimation(parent: animation, curve: _kCurve, reverseCurve: _kCurve.flipped));

    final secondaryPosition = Tween<Offset>(begin: Offset.zero, end: const Offset(-0.15, 0)).animate(
      linear
          ? secondaryAnimation
          : CurvedAnimation(parent: secondaryAnimation, curve: _kCurve, reverseCurve: _kCurve.flipped),
    );

    return _BackGestureDetector(
      enabledCallback: () => _canPopWithGesture,
      controller: controller!,
      navigator: navigator!,
      child: AnimatedBuilder(
        animation: secondaryAnimation,
        builder: (context, child) => DecoratedBox(
          decoration: BoxDecoration(color: Color.fromRGBO(0, 0, 0, 0.1 * secondaryAnimation.value)),
          position: DecorationPosition.foreground,
          child: SlideTransition(position: secondaryPosition, child: child),
        ),
        child: SlideTransition(
          position: primaryPosition,
          child: DecoratedBox(
            decoration: const BoxDecoration(
              boxShadow: [BoxShadow(color: Color.fromRGBO(0, 0, 0, 0.15), blurRadius: 24, offset: Offset(-4, 0))],
            ),
            child: AnimatedBuilder(
              animation: Listenable.merge([animation, secondaryAnimation]),
              builder: (context, child) {
                final t = ((1 - animation.value) + secondaryAnimation.value).clamp(0.0, 1.0);
                // t가 0.05 이상이면 거의 풀 radius 유지, 끝에서만 급격히 0으로
                final radius = t < 0.05 ? t / 0.05 * _kCornerRadius : _kCornerRadius;
                return ClipRRect(borderRadius: BorderRadius.circular(radius), child: child);
              },
              child: child,
            ),
          ),
        ),
      ),
    );
  }
}

class _BackGestureDetector extends StatefulWidget {
  const _BackGestureDetector({
    required this.child,
    required this.enabledCallback,
    required this.controller,
    required this.navigator,
  });

  final Widget child;
  final ValueGetter<bool> enabledCallback;
  final AnimationController controller;
  final NavigatorState navigator;

  @override
  State<_BackGestureDetector> createState() => _BackGestureDetectorState();
}

class _BackGestureDetectorState extends State<_BackGestureDetector> {
  late final HorizontalDragGestureRecognizer _recognizer;
  bool _active = false;

  @override
  void initState() {
    super.initState();
    _recognizer = HorizontalDragGestureRecognizer(debugOwner: this)
      ..onStart = _onStart
      ..onUpdate = _onUpdate
      ..onEnd = _onEnd
      ..onCancel = _onCancel;
  }

  @override
  void dispose() {
    _recognizer.dispose();
    super.dispose();
  }

  void _onPointerDown(PointerDownEvent event) {
    if (widget.enabledCallback()) {
      _recognizer.addPointer(event);
    }
  }

  void _onStart(DragStartDetails details) {
    _active = true;
    widget.navigator.didStartUserGesture();
  }

  void _onUpdate(DragUpdateDetails details) {
    if (_active) {
      widget.controller.value -= details.primaryDelta! / context.size!.width;
    }
  }

  void _onEnd(DragEndDetails details) {
    if (!_active) {
      return;
    }

    _active = false;

    final velocity = details.velocity.pixelsPerSecond.dx / context.size!.width;
    final shouldPop = velocity.abs() >= _kMinFlingVelocity ? velocity > 0 : widget.controller.value < 0.5;

    if (shouldPop) {
      // pop() calls controller.reverse() internally → AnimationStatus.dismissed.
      // Do NOT use animateTo(0) as it produces AnimationStatus.completed at value 0,
      // leaving an invisible opaque overlay that blocks everything behind it.
      widget.navigator.pop();
    } else {
      unawaited(
        widget.controller.animateTo(
          1,
          duration: const Duration(milliseconds: 300),
          curve: Curves.fastLinearToSlowEaseIn,
        ),
      );
    }

    _endGesture();
  }

  void _onCancel() {
    if (!_active) {
      return;
    }

    _active = false;
    unawaited(
      widget.controller.animateTo(1, duration: const Duration(milliseconds: 300), curve: Curves.fastLinearToSlowEaseIn),
    );
    _endGesture();
  }

  void _endGesture() {
    // Stop immediately so buildTransitions switches from linear to curved
    // for the completion animation.
    widget.navigator.didStopUserGesture();
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      fit: StackFit.passthrough,
      children: [
        widget.child,
        Positioned(
          left: 0,
          top: 0,
          bottom: 0,
          width: _kBackGestureWidth,
          child: Listener(onPointerDown: _onPointerDown, behavior: HitTestBehavior.translucent),
        ),
      ],
    );
  }
}
