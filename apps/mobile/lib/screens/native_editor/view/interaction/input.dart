import 'package:typie/screens/native_editor/view/interaction/mode.dart';

sealed class InteractionInput {
  const InteractionInput();
}

class PointerDownInput extends InteractionInput {
  const PointerDownInput({required this.pointer});

  final int pointer;
}

class PointerMoveInput extends InteractionInput {
  const PointerMoveInput({required this.pointer});

  final int pointer;
}

class PointerUpInput extends InteractionInput {
  const PointerUpInput({required this.pointer});

  final int pointer;
}

class PointerCancelInput extends InteractionInput {
  const PointerCancelInput({required this.pointer});

  final int pointer;
}

class PanStartInput extends InteractionInput {
  const PanStartInput();
}

class PanEndInput extends InteractionInput {
  const PanEndInput();
}

class PanCancelInput extends InteractionInput {
  const PanCancelInput();
}

class LongPressStartInput extends InteractionInput {
  const LongPressStartInput();
}

class LongPressEndInput extends InteractionInput {
  const LongPressEndInput();
}

class PinchStartInput extends InteractionInput {
  const PinchStartInput();
}

class PinchEndInput extends InteractionInput {
  const PinchEndInput();
}

class TextHandleDragStartInput extends InteractionInput {
  const TextHandleDragStartInput();
}

class TextHandleDragEndInput extends InteractionInput {
  const TextHandleDragEndInput();
}

class DoubleTapDragStartInput extends InteractionInput {
  const DoubleTapDragStartInput();
}

class DoubleTapDragEndInput extends InteractionInput {
  const DoubleTapDragEndInput();
}

class TableHandleDragStartInput extends InteractionInput {
  const TableHandleDragStartInput();
}

class TableHandleDragEndInput extends InteractionInput {
  const TableHandleDragEndInput();
}

class DndStartInput extends InteractionInput {
  const DndStartInput({required this.local});

  final bool local;
}

class DndEnterInput extends InteractionInput {
  const DndEnterInput();
}

class DndOverInput extends InteractionInput {
  const DndOverInput();
}

class DndLeaveInput extends InteractionInput {
  const DndLeaveInput();
}

class DndDropInput extends InteractionInput {
  const DndDropInput();
}

class DndSessionEndInput extends InteractionInput {
  const DndSessionEndInput();
}

class AuxiliaryGestureStartInput extends InteractionInput {
  const AuxiliaryGestureStartInput({required this.kind});

  final AuxiliaryGestureKind kind;
}

class AuxiliaryGestureUpdateInput extends InteractionInput {
  const AuxiliaryGestureUpdateInput({required this.kind});

  final AuxiliaryGestureKind kind;
}

class AuxiliaryGestureEndInput extends InteractionInput {
  const AuxiliaryGestureEndInput();
}
