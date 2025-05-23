import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/styles/colors.dart';

enum ToastType { success, error, notification }

class _Widget extends HookWidget {
  const _Widget({required this.type, required this.message, required this.duration, required this.completer});

  final Completer<void> completer;

  final ToastType type;
  final String message;
  final Duration duration;

  @override
  Widget build(BuildContext context) {
    final animationController = useAnimationController(duration: const Duration(milliseconds: 200));
    final tweenedOpacity = useMemoized(() {
      final curve = CurvedAnimation(parent: animationController, curve: Curves.easeOut, reverseCurve: Curves.easeIn);
      return Tween<double>(begin: 0, end: 1).animate(curve);
    }, [animationController]);
    final tweenedOffset = useMemoized(() {
      final curve = CurvedAnimation(parent: animationController, curve: Curves.easeOut, reverseCurve: Curves.easeIn);
      return Tween<Offset>(begin: const Offset(0, 0.2), end: Offset.zero).animate(curve);
    }, [animationController]);

    useEffect(() {
      animationController.forward();

      final timer = Timer(duration, () async {
        await animationController.reverse().then((_) {
          completer.complete();
        });
      });

      return timer.cancel;
    }, [animationController]);

    final mediaQuery = MediaQuery.of(context);
    final safeAreaBottom = mediaQuery.padding.bottom;
    final keyboardHeight = mediaQuery.viewInsets.bottom;

    return Positioned(
      bottom: safeAreaBottom + keyboardHeight + 12,
      left: 24,
      right: 24,
      child: Material(
        type: MaterialType.transparency,
        child: SlideTransition(
          position: tweenedOffset,
          child: FadeTransition(
            opacity: tweenedOpacity,
            child: Box(
              decoration: BoxDecoration(color: AppColors.gray_950, borderRadius: BorderRadius.circular(999)),
              padding: const Pad(all: 12),
              child: Row(
                children: [
                  Box(
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(999),
                      color: switch (type) {
                        ToastType.success => AppColors.green_600,
                        ToastType.error => AppColors.red_600,
                        ToastType.notification => AppColors.blue_600,
                      },
                    ),
                    width: 20,
                    height: 20,
                    child: Center(
                      child: switch (type) {
                        ToastType.success => const Icon(LucideIcons.check, color: AppColors.white, size: 14),
                        ToastType.error => const Icon(TypieIcons.exclamation, color: AppColors.white, size: 14),
                        ToastType.notification => const Icon(LucideIcons.bell, color: AppColors.white, size: 14),
                      },
                    ),
                  ),
                  const Box.gap(8),
                  Expanded(
                    child: Text(
                      message,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: AppColors.white),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

extension ToastExtension on BuildContext {
  static OverlayEntry? _entry;

  void toast(ToastType type, String message, {Duration? duration}) {
    if (_entry != null) {
      _entry?.remove();
    }

    final completer = Completer<void>();

    _entry = OverlayEntry(
      builder: (context) {
        return _Widget(
          type: type,
          message: message,
          duration: duration ?? const Duration(seconds: 2),
          completer: completer,
        );
      },
    );

    Overlay.of(this, rootOverlay: true).insert(_entry!);

    unawaited(
      completer.future.then((_) {
        _entry?.remove();
        _entry = null;
      }),
    );
  }
}
