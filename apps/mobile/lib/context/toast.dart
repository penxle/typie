import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/widgets/responsive_container.dart';

enum ToastType { success, error, notification }

class _Widget extends HookWidget {
  const _Widget({
    required this.type,
    required this.message,
    required this.bottom,
    required this.duration,
    required this.completer,
  });

  final Completer<void> completer;

  final ToastType type;
  final String message;
  final double bottom;
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
      unawaited(animationController.forward());

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
      bottom: safeAreaBottom + keyboardHeight + bottom,
      left: 0,
      right: 0,
      child: Material(
        type: MaterialType.transparency,
        child: ResponsiveContainer(
          child: Padding(
            padding: const Pad(horizontal: 24),
            child: SlideTransition(
              position: tweenedOffset,
              child: FadeTransition(
                opacity: tweenedOpacity,
                child: Container(
                  decoration: BoxDecoration(
                    color: context.colors.surfaceDark,
                    borderRadius: BorderRadius.circular(999),
                  ),
                  padding: const Pad(all: 12),
                  child: Row(
                    children: [
                      Container(
                        width: 20,
                        height: 20,
                        decoration: BoxDecoration(
                          borderRadius: BorderRadius.circular(999),
                          color: switch (type) {
                            ToastType.success => context.colors.accentSuccess,
                            ToastType.error => context.colors.accentDanger,
                            ToastType.notification => context.colors.accentSuccess,
                          },
                        ),
                        child: Center(
                          child: switch (type) {
                            ToastType.success => Icon(
                              LucideLightIcons.check,
                              color: context.colors.textBright,
                              size: 12,
                            ),
                            ToastType.error => Icon(TypieIcons.exclamation, color: context.colors.textBright, size: 12),
                            ToastType.notification => Icon(
                              LucideLightIcons.bell,
                              color: context.colors.textBright,
                              size: 12,
                            ),
                          },
                        ),
                      ),
                      const Gap(8),
                      Expanded(
                        child: Text(
                          message,
                          style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textBright),
                          overflow: TextOverflow.ellipsis,
                          maxLines: 1,
                        ),
                      ),
                    ],
                  ),
                ),
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

  void toast(ToastType type, String message, {Duration duration = const Duration(seconds: 2), double bottom = 12}) {
    if (_entry != null) {
      _entry?.remove();
    }

    final completer = Completer<void>();

    _entry = OverlayEntry(
      builder: (context) {
        return _Widget(type: type, message: message, bottom: bottom, duration: duration, completer: completer);
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
