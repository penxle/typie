import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';

void handleHtmlPasted(EditorController controller, Map<String, dynamic> cmd) {
  final text = cmd['text'] as String;
  final from = cmd['from'] as Map<String, dynamic>;
  final to = cmd['to'] as Map<String, dynamic>;

  controller.updateState(
    (state) => state.copyWith(
      pasteOptions: PasteOptionsInfo(text: text, from: from, to: to),
    ),
  );
}
