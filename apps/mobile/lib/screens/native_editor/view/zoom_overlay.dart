import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';

const _zoomOverlayVisibleDuration = Duration(seconds: 1);
const _zoomOverlayFadeDuration = Duration(milliseconds: 180);

class NativeEditorZoomOverlay extends HookWidget {
  const NativeEditorZoomOverlay({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final state = useListenable(scope.controller);
    final layout = state.state.layout;
    final displayZoom = useValueListenable(scope.displayZoom);

    final isVisible = useState(false);
    final hideTimer = useRef<Timer?>(null);
    final lastZoom = useRef<double?>(null);

    void showTemporarily() {
      if (!isVisible.value) {
        isVisible.value = true;
      }
      hideTimer.value?.cancel();
      hideTimer.value = Timer(_zoomOverlayVisibleDuration, () {
        if (!context.mounted) {
          return;
        }
        isVisible.value = false;
      });
    }

    useEffect(() {
      final prevZoom = lastZoom.value;
      lastZoom.value = displayZoom;
      final shouldShow = prevZoom == null || zoomDiffers(prevZoom, displayZoom);
      if (shouldShow) {
        showTemporarily();
      }
      return null;
    }, [displayZoom]);

    useEffect(() {
      return () {
        hideTimer.value?.cancel();
        hideTimer.value = null;
      };
    }, const []);

    if (layout is! PaginatedLayout) {
      return const SizedBox.shrink();
    }

    final zoomPercent = (displayZoom * 100).round();
    return Positioned(
      left: 20,
      bottom: 20,
      child: IgnorePointer(
        child: AnimatedOpacity(
          opacity: isVisible.value ? 1 : 0,
          duration: _zoomOverlayFadeDuration,
          curve: Curves.easeOut,
          child: Material(
            color: Colors.transparent,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: context.colors.surfaceSubtle.withValues(alpha: 0.95),
                border: Border.all(color: context.colors.borderStrong),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                child: Text(
                  '$zoomPercent%',
                  style: TextStyle(fontSize: 12, fontWeight: FontWeight.w500, color: context.colors.textDefault),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
