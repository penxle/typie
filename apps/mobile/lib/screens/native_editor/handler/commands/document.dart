import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/state/controller.dart';

void handleDocChanged(EditorController controller, Map<String, dynamic> cmd) {
  controller.onDocChanged?.call();
  controller.typewriterNeedsScroll = true;
}

void handleExitedDocumentStart(EditorController controller, Map<String, dynamic> cmd) {
  controller.onExitedDocumentStart?.call();
}

void handleExternalElements(EditorController controller, Map<String, dynamic> cmd) {
  final elements = cmd['elements'] as List<dynamic>;
  controller.updateState(
    (state) => state.copyWith(
      externalElements: elements.map((e) => ExternalElement.fromJson(e as Map<String, dynamic>)).toList(),
    ),
  );
}
