import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';

void handlePlaceholderChanged(EditorController controller, Map<String, dynamic> cmd) {
  final visible = cmd['visible'] as bool? ?? false;
  final bounds = cmd['bounds'] as Map<String, dynamic>?;

  controller.updateState(
    (state) => state.copyWith(
      placeholder: PlaceholderInfo(
        visible: visible,
        x: (bounds?['x'] as num?)?.toDouble(),
        y: (bounds?['y'] as num?)?.toDouble(),
        width: (bounds?['width'] as num?)?.toDouble(),
        height: (bounds?['height'] as num?)?.toDouble(),
      ),
    ),
  );
}
