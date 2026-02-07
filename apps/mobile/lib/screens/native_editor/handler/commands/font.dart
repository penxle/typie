import 'dart:async';

import 'package:typie/screens/native_editor/state/controller.dart';

void handleFontRequired(EditorController controller, Map<String, dynamic> cmd) {
  final manager = controller.fontManager;
  if (manager == null) {
    return;
  }

  final family = cmd['family'] as String;
  final weight = (cmd['weight'] as num).toInt();
  final codepoints = (cmd['codepoints'] as List<dynamic>).cast<int>();

  unawaited(
    manager.ensureRequiredFont(family, weight, codepoints).then((_) {
      controller.dispatch({'type': 'fontsLoaded'});
    }),
  );
}

void handleFallbackFontRequired(EditorController controller, Map<String, dynamic> cmd) {
  final manager = controller.fontManager;
  if (manager == null) {
    return;
  }

  final codepoints = (cmd['codepoints'] as List<dynamic>).cast<int>();
  if (codepoints.isEmpty) {
    return;
  }

  unawaited(
    manager.ensureRequiredFallbackFont(codepoints).then((_) {
      controller.dispatch({'type': 'fontsLoaded'});
    }),
  );
}
