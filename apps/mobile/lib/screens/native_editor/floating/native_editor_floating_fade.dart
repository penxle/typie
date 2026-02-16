import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/services/preference.dart';

class NativeEditorFloatingFadeController {
  NativeEditorFloatingFadeController({required this.opacity, required this.showImmediately});
  final AnimationController opacity;
  final VoidCallback showImmediately;
}

NativeEditorFloatingFadeController useNativeEditorFloatingFade() {
  final pref = useService<Pref>();
  final scope = ContentScope.of(useContext());

  final opacity = useAnimationController(
    initialValue: 1,
    duration: const Duration(milliseconds: 200),
    lowerBound: 0.05,
  );

  final idleTimer = useRef<Timer?>(null);
  final isFadedOut = useRef(false);

  void showImmediately() {
    idleTimer.value?.cancel();
    unawaited(opacity.forward());
    isFadedOut.value = false;
  }

  void scheduleFadeIn() {
    if (!pref.widgetAutoFadeEnabled) {
      return;
    }
    idleTimer.value?.cancel();
    idleTimer.value = Timer(const Duration(milliseconds: 1500), () {
      unawaited(opacity.forward());
      isFadedOut.value = false;
    });
  }

  void startFadeOut() {
    if (!pref.widgetAutoFadeEnabled) {
      return;
    }
    idleTimer.value?.cancel();

    if (!isFadedOut.value) {
      unawaited(opacity.reverse());
      isFadedOut.value = true;
    }

    scheduleFadeIn();
  }

  useEffect(() {
    if (pref.widgetAutoFadeEnabled) {
      EditorSelection? previousSelection;
      CursorInfo? previousCursor;

      void onEditorStateChange() {
        final currentSelection = scope.controller.state.selection;
        final currentCursor = scope.controller.state.cursor;

        final selectionChanged =
            currentSelection != null && previousSelection != null && (previousSelection != currentSelection);
        final cursorChanged =
            currentCursor != null && previousCursor != null && !previousCursor!.isSamePosition(currentCursor);

        if (selectionChanged || cursorChanged) {
          startFadeOut();
        }

        previousSelection = currentSelection;
        previousCursor = currentCursor;
      }

      void onScrollActivity() {
        startFadeOut();
      }

      scope.controller.addListener(onEditorStateChange);
      scope.verticalScrollController.addListener(onScrollActivity);

      return () {
        scope.controller.removeListener(onEditorStateChange);
        scope.verticalScrollController.removeListener(onScrollActivity);
        idleTimer.value?.cancel();
      };
    } else {
      idleTimer.value?.cancel();
      unawaited(opacity.forward());
      isFadedOut.value = false;
    }
    return null;
  }, [pref.widgetAutoFadeEnabled, scope.controller, scope.verticalScrollController]);

  return NativeEditorFloatingFadeController(opacity: opacity, showImmediately: showImmediately);
}
