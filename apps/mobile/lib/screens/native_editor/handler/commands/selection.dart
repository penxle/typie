import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';

void handleMarksChanged(EditorController controller, Map<String, dynamic> cmd) {
  final uniform = cmd['uniformMarks'] as List<dynamic>;
  final mixed = cmd['mixedMarks'] as List<dynamic>;

  controller.updateState(
    (state) => state.copyWith(uniformMarks: uniform.cast<Map<String, dynamic>>(), mixedMarks: mixed.cast<String>()),
  );
}

void handleSelectionChanged(EditorController controller, Map<String, dynamic> cmd) {
  final stats = cmd['stats'] as Map<String, dynamic>;
  final fromHandleMap = cmd['fromHandle'] as Map<String, dynamic>?;
  final toHandleMap = cmd['toHandle'] as Map<String, dynamic>?;

  final fromHandle = fromHandleMap != null ? SelectionHandleInfo.fromMap(fromHandleMap) : null;
  final toHandle = toHandleMap != null ? SelectionHandleInfo.fromMap(toHandleMap) : null;

  controller.updateState((state) => state.copyWith(selectionStats: stats, fromHandle: fromHandle, toHandle: toHandle));

  final anchor = cmd['anchor'] as Map<String, dynamic>?;
  final head = cmd['head'] as Map<String, dynamic>?;
  if (anchor != null && head != null) {
    controller.onSelectionChanged?.call(anchor, head);
  }
}
