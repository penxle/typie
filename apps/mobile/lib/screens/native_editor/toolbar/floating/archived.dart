import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/toolbar/buttons/floating.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';

class NativeEditorArchivedFloatingToolbar extends HookWidget {
  const NativeEditorArchivedFloatingToolbar({required this.element, super.key});

  final ExternalElement element;

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final uploadManager = scope.uploadManager;

    useListenable(uploadManager);

    final archivedData = element.data as ArchivedElementData;
    final asset = archivedData.id != null ? uploadManager.archivedAssets[archivedData.id] : null;

    if (asset == null) {
      return const SizedBox.shrink();
    }

    return FloatingToolbarButton(
      icon: LucideLightIcons.eye,
      onTap: () async {
        await context.showBottomSheet(
          child: AppFullBottomSheet(
            title: '보관된 블록',
            child: SingleChildScrollView(
              child: SelectableText(
                asset.content,
                style: TextStyle(fontSize: 13, fontFamily: 'monospace', color: context.colors.textSubtle),
              ),
            ),
          ),
        );
      },
    );
  }
}
