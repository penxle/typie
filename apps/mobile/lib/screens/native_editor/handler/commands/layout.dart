import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';

void handleLayoutChanged(EditorController controller, Map<String, dynamic> cmd) {
  final pageCount = cmd['pageCount'] as int;
  final layoutModeMap = cmd['layoutMode'] as Map<String, dynamic>;
  final pageWidth = (cmd['pageWidth'] as num).toDouble();
  final pageHeights = cmd['pageHeights'] as List<dynamic>;

  final isPaginated = layoutModeMap['type'] == 'paginated';
  final LayoutModeInfo layoutMode;

  if (isPaginated) {
    layoutMode = LayoutModeInfo.paginated(
      pageWidth: (layoutModeMap['pageWidth'] as num).toDouble(),
      pageHeight: (layoutModeMap['pageHeight'] as num).toDouble(),
      pageMarginTop: (layoutModeMap['pageMarginTop'] as num).toDouble(),
      pageMarginBottom: (layoutModeMap['pageMarginBottom'] as num).toDouble(),
      pageMarginLeft: (layoutModeMap['pageMarginLeft'] as num).toDouble(),
      pageMarginRight: (layoutModeMap['pageMarginRight'] as num).toDouble(),
    );
  } else {
    layoutMode = LayoutModeInfo.continuous(maxWidth: (layoutModeMap['maxWidth'] as num).toDouble());
  }

  final hadLayout = controller.state.layout != null;

  controller.updateState(
    (state) => state.copyWith(
      layout: LayoutInfo(
        pageCount: pageCount,
        isPaginated: isPaginated,
        pageWidth: pageWidth,
        pageHeights: pageHeights.cast<num>().map((e) => e.toDouble()).toList(),
        layoutMode: layoutMode,
      ),
      renderVersion: Object(),
    ),
  );

  if (!hadLayout && pageCount > 0) {
    controller.onEditorReady?.call();
  }
}

void handleSettingsChanged(EditorController controller, Map<String, dynamic> cmd) {
  final paragraphIndent = (cmd['paragraphIndent'] as num).toDouble();
  final blockGap = (cmd['blockGap'] as num).toDouble();

  controller.updateState(
    (state) => state.copyWith(
      settings: state.settings.copyWith(paragraphIndent: paragraphIndent, blockGap: blockGap),
    ),
  );
}

void handleRenderRequired(EditorController controller, Map<String, dynamic> cmd) {
  controller.updateState((state) => state.copyWith(renderVersion: Object()));
}
