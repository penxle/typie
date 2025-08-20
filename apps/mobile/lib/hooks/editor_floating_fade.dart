import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/services/preference.dart';

class EditorFloatingFadeController {
  EditorFloatingFadeController({required this.opacity, required this.showImmediately});
  final AnimationController opacity;
  final VoidCallback showImmediately;
}

EditorFloatingFadeController useEditorFloatingFade() {
  final pref = useService<Pref>();
  final scope = EditorStateScope.of(useContext());

  final opacity = useAnimationController(
    initialValue: 1,
    duration: const Duration(milliseconds: 200),
    lowerBound: 0.05,
  );

  final idleTimer = useRef<Timer?>(null);
  final isFadedOut = useRef(false);

  void showImmediately() {
    idleTimer.value?.cancel();
    opacity.forward();
    isFadedOut.value = false;
  }

  void scheduleFadeIn() {
    if (!pref.widgetAutoFadeEnabled) {
      return;
    }
    idleTimer.value?.cancel();
    idleTimer.value = Timer(const Duration(milliseconds: 1500), () {
      opacity.forward();
      isFadedOut.value = false;
    });
  }

  void startFadeOut() {
    if (!pref.widgetAutoFadeEnabled) {
      return;
    }
    idleTimer.value?.cancel();

    if (!isFadedOut.value) {
      opacity.reverse();
      isFadedOut.value = true;
    }

    scheduleFadeIn();
  }

  useEffect(() {
    if (pref.widgetAutoFadeEnabled) {
      int? previousFrom;
      int? previousTo;

      void onProseMirrorStateChange() {
        final currentState = scope.proseMirrorState.value;
        if (currentState != null) {
          final currentSelection = currentState.selection;

          int? currentFrom;
          int? currentTo;

          switch (currentSelection) {
            case ProseMirrorTextSelection(:final from, :final to):
              currentFrom = from;
              currentTo = to;
            case ProseMirrorNodeSelection(:final anchor):
              currentFrom = anchor;
              currentTo = anchor;
            case ProseMirrorMultiNodeSelection(:final anchor, :final head):
              currentFrom = anchor < head ? anchor : head;
              currentTo = anchor > head ? anchor : head;
            case ProseMirrorAllSelection():
            case ProseMirrorCellSelection():
              break;
          }

          // NOTE: 셀렉션이 변경되었을 때만 페이드 아웃. 초기 포커스 또는 blur 시 페이드 아웃하지 않음.
          if (currentFrom != null &&
              currentTo != null &&
              previousFrom != null &&
              previousTo != null &&
              (previousFrom != currentFrom || previousTo != currentTo)) {
            startFadeOut();
          }

          previousFrom = currentFrom;
          previousTo = currentTo;
        }
      }

      void onScrollActivity() {
        startFadeOut();
      }

      scope.proseMirrorState.addListener(onProseMirrorStateChange);
      scope.scrollTop.addListener(onScrollActivity);

      return () {
        scope.proseMirrorState.removeListener(onProseMirrorStateChange);
        scope.scrollTop.removeListener(onScrollActivity);
        idleTimer.value?.cancel();
      };
    } else {
      // NOTE: 자동 페이드 끔
      idleTimer.value?.cancel();
      opacity.reverse();
      isFadedOut.value = false;
    }
    return null;
  }, [pref.widgetAutoFadeEnabled, scope]);

  return EditorFloatingFadeController(opacity: opacity, showImmediately: showImmediately);
}
