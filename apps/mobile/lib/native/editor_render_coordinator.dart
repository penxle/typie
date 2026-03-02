import 'dart:async';

final class EditorRenderCoordinator {
  EditorRenderCoordinator._();

  static final _inFlightByEditor = <int, int>{};
  static final _waitersByEditor = <int, List<Completer<void>>>{};
  static final _disposingEditors = <int>{};

  static bool isEditorDisposing(int editorPtr) {
    return _disposingEditors.contains(editorPtr);
  }

  static void markEditorDisposing(int editorPtr) {
    _disposingEditors.add(editorPtr);
  }

  static void markEditorDisposed(int editorPtr) {
    _disposingEditors.remove(editorPtr);
    _completeWaiters(editorPtr);
  }

  static void markBatchStarted(Iterable<int> editorPtrs) {
    for (final editorPtr in editorPtrs.toSet()) {
      _inFlightByEditor[editorPtr] = (_inFlightByEditor[editorPtr] ?? 0) + 1;
    }
  }

  static void markBatchFinished(Iterable<int> editorPtrs) {
    for (final editorPtr in editorPtrs.toSet()) {
      final current = _inFlightByEditor[editorPtr];
      if (current == null || current <= 1) {
        _inFlightByEditor.remove(editorPtr);
      } else {
        _inFlightByEditor[editorPtr] = current - 1;
      }
      if ((_inFlightByEditor[editorPtr] ?? 0) == 0) {
        _completeWaiters(editorPtr);
      }
    }
  }

  static Future<void> waitForEditorIdle(int editorPtr) {
    if ((_inFlightByEditor[editorPtr] ?? 0) == 0) {
      return Future<void>.value();
    }

    final completer = Completer<void>();
    (_waitersByEditor[editorPtr] ??= []).add(completer);
    return completer.future;
  }

  static void _completeWaiters(int editorPtr) {
    final waiters = _waitersByEditor.remove(editorPtr);
    if (waiters == null) {
      return;
    }
    for (final waiter in waiters) {
      if (!waiter.isCompleted) {
        waiter.complete();
      }
    }
  }
}
