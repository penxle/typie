import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';

void handleCursorChanged(EditorController controller, Map<String, dynamic> cmd) {
  final cursor = CursorInfo.fromMap(cmd);
  controller.updateState((state) => state.copyWith(cursor: cursor));
}
