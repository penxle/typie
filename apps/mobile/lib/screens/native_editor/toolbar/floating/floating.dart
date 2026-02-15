import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/toolbar/floating/archived.dart';
import 'package:typie/screens/native_editor/toolbar/floating/blockquote.dart';
import 'package:typie/screens/native_editor/toolbar/floating/callout.dart';
import 'package:typie/screens/native_editor/toolbar/floating/embed.dart';
import 'package:typie/screens/native_editor/toolbar/floating/file.dart';
import 'package:typie/screens/native_editor/toolbar/floating/fold.dart';
import 'package:typie/screens/native_editor/toolbar/floating/horizontal_rule.dart';
import 'package:typie/screens/native_editor/toolbar/floating/image.dart';
import 'package:typie/screens/native_editor/toolbar/floating/list.dart';
import 'package:typie/screens/native_editor/toolbar/floating/table.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';

class NativeEditorFloatingToolbar extends HookWidget {
  const NativeEditorFloatingToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final floatingContext = useValueListenable(scope.floatingContext);
    final floatingNodeId = useValueListenable(scope.floatingNodeId);
    final elements = useValueListenable(scope.externalElements);

    final selectedElement = floatingNodeId == null
        ? null
        : elements.where((element) => element.nodeId == floatingNodeId).firstOrNull;

    final tableOverlays = useValueListenable(scope.controller.tableOverlays);
    final selectedTable = floatingNodeId == null
        ? null
        : tableOverlays.where((overlay) => overlay.tableId == floatingNodeId).firstOrNull;

    final selectedElementToolbars = <String, Widget Function()>{
      'selected_image': () => NativeEditorImageFloatingToolbar(element: selectedElement!),
      'selected_file': () => NativeEditorFileFloatingToolbar(element: selectedElement!),
      'selected_embed': () => NativeEditorEmbedFloatingToolbar(element: selectedElement!),
      'selected_archived': () => NativeEditorArchivedFloatingToolbar(element: selectedElement!),
    };

    if (selectedElement != null) {
      final selectedElementToolbarBuilder = selectedElementToolbars[floatingContext];
      if (selectedElementToolbarBuilder != null) {
        return selectedElementToolbarBuilder();
      }
    }

    return switch (floatingContext) {
      'selected_horizontal_rule' => const NativeEditorHorizontalRuleFloatingToolbar(),
      'in_bullet_list' || 'in_ordered_list' => const NativeEditorListFloatingToolbar(),
      'in_blockquote' => const NativeEditorBlockquoteFloatingToolbar(),
      'in_callout' => const NativeEditorCalloutFloatingToolbar(),
      'in_fold' => const NativeEditorFoldFloatingToolbar(),
      'in_table' || 'selected_table' =>
        selectedTable != null ? NativeEditorTableFloatingToolbar(table: selectedTable) : const SizedBox.shrink(),
      _ => const SizedBox.shrink(),
    };
  }
}
