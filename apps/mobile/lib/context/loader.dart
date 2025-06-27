import 'dart:async';

import 'package:back_button_interceptor/back_button_interceptor.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';

extension LoaderExtension on BuildContext {
  static OverlayEntry? _entry;
  static ValueNotifier<bool>? _shouldDismiss;
  static Completer<void>? _animationCompleter;

  Future<T> runWithLoader<T>(Future<T> Function() fn) async {
    BackButtonInterceptor.add(_backButtonInterceptor);

    try {
      if (_shouldDismiss != null) {
        _shouldDismiss!.value = true;
        await _animationCompleter?.future;
      }

      _shouldDismiss = ValueNotifier(false);
      _animationCompleter = Completer<void>();

      try {
        _entry = OverlayEntry(
          builder: (context) {
            return _Widget(
              shouldDismiss: _shouldDismiss!,
              onAnimationComplete: () {
                _entry?.remove();
                _entry = null;
                _animationCompleter!.complete();
              },
            );
          },
        );

        Overlay.of(this, rootOverlay: true).insert(_entry!);

        return await fn();
      } finally {
        _shouldDismiss?.value = true;

        await _animationCompleter?.future;

        _shouldDismiss = null;
        _animationCompleter = null;
      }
    } finally {
      BackButtonInterceptor.remove(_backButtonInterceptor);
    }
  }
}

class _Widget extends HookWidget {
  const _Widget({required this.onAnimationComplete, required this.shouldDismiss});

  final VoidCallback onAnimationComplete;
  final ValueNotifier<bool> shouldDismiss;

  @override
  Widget build(BuildContext context) {
    final animationController = useAnimationController(duration: const Duration(milliseconds: 200));

    final tweenedOpacity = useMemoized(() {
      final curve = CurvedAnimation(parent: animationController, curve: Curves.easeOut, reverseCurve: Curves.easeIn);
      return Tween<double>(begin: 0, end: 1).animate(curve);
    }, [animationController]);

    useOnListenableChange(shouldDismiss, () {
      if (shouldDismiss.value && context.mounted) {
        animationController.reverse();
      }
    });

    useEffect(() {
      void handleStatusChange(AnimationStatus status) {
        if (status == AnimationStatus.dismissed) {
          onAnimationComplete();
        }
      }

      animationController.addStatusListener(handleStatusChange);

      return () {
        animationController.removeStatusListener(handleStatusChange);
      };
    }, [animationController, onAnimationComplete]);

    useEffect(() {
      if (shouldDismiss.value) {
        onAnimationComplete();
      } else {
        animationController.forward();
      }

      return null;
    }, [animationController]);

    return FadeTransition(
      opacity: tweenedOpacity,
      child: Stack(
        children: [
          ModalBarrier(dismissible: false, color: context.colors.overlayDefault.withValues(alpha: 0.5)),
          Center(child: CircularProgressIndicator(color: context.colors.textDefault)),
        ],
      ),
    );
  }
}

bool _backButtonInterceptor(bool stopDefaultButtonEvent, RouteInfo routeInfo) {
  return true;
}
