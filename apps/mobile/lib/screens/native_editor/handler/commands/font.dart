import 'dart:async';

import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/fonts.dart';

void handleFontsRequired(EditorController controller, Map<String, dynamic> cmd) {
  final manager = controller.fontManager;
  if (manager == null || manager.pendingFontLoad) {
    return;
  }

  final fonts = cmd['fonts'] as List<dynamic>;
  manager.pendingFontLoad = true;
  final fontList = fonts.cast<List<dynamic>>().map((f) => (f[0] as String, (f[1] as num).toInt())).toList();

  unawaited(
    manager
        .ensureRequiredFonts(
          fontList,
          callbacks: (onStart: controller.incrementFontLoading, onEnd: controller.decrementFontLoading),
        )
        .then((loaded) {
          manager.pendingFontLoad = false;
          if (loaded) {
            controller.dispatch({'type': 'fontsLoaded'});
          }
        }),
  );
}

void handleWritingSystem(EditorController controller, Map<String, dynamic> cmd) {
  final manager = controller.fontManager;
  if (manager == null) {
    return;
  }

  final systems = cmd['systems'] as List<dynamic>;
  final systemList = systems.cast<String>().map((s) => WritingSystem.values.firstWhere((ws) => ws.name == s)).toList();

  unawaited(
    manager
        .ensureRequiredWritingSystems(
          systemList,
          callbacks: (onStart: controller.incrementFontLoading, onEnd: controller.decrementFontLoading),
        )
        .then((loaded) {
          if (loaded) {
            controller.dispatch({'type': 'fontsLoaded'});
          }
        }),
  );
}
