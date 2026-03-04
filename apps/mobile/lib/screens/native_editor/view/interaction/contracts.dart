abstract interface class ResettableInteraction {
  void reset();
}

abstract interface class InteractionGesture implements ResettableInteraction {}

abstract interface class InteractionSemantic implements ResettableInteraction {}
