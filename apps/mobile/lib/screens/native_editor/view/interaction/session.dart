abstract interface class InteractionSession {
  void reset();
}

abstract class DisposableInteractionSession implements InteractionSession {
  void dispose() {
    reset();
  }
}
