import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/base.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/widgets/svg_image.dart';

class _Node {
  const _Node({required this.icon, required this.label, required this.type, this.attrs});

  final String icon;
  final String label;
  final String type;
  final Map<String, dynamic>? attrs;
}

class NativeEditorInsertBottomToolbar extends HookWidget {
  const NativeEditorInsertBottomToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final keyboardType = useValueListenable(scope.keyboardType);

    final nodes = [
      const _Node(icon: 'image', label: '이미지', type: 'insertImage'),
      const _Node(icon: 'paperclip', label: '파일', type: 'insertFile'),
      const _Node(icon: 'file-up', label: '임베드', type: 'insertEmbed'),
      _Node(
        icon: 'horizontal-rule',
        label: '구분선',
        type: 'horizontalRule',
        attrs: {'type': editorDefaultValues['horizontalRule']},
      ),
      _Node(icon: 'quote', label: '인용구', type: 'blockquote', attrs: {'type': editorDefaultValues['blockquote']}),
      const _Node(icon: 'gallery-vertical-end', label: '강조', type: 'toggleCallout'),
      const _Node(icon: 'chevrons-down-up', label: '접기', type: 'insertFold'),
      const _Node(icon: 'table', label: '표', type: 'table'),
      const _Node(icon: 'list', label: '목록', type: 'toggleBulletList'),
    ];

    return GridView.extent(
      maxCrossAxisExtent: 96,
      padding: Pad(all: 16, bottom: MediaQuery.paddingOf(context).bottom),
      mainAxisSpacing: 16,
      crossAxisSpacing: 16,
      children: nodes.map((node) {
        return ToolbarButton(
          onTap: () {
            if ((node.type == 'insertImage' || node.type == 'insertFile') && scope.controller.restrictedBlob) {
              scope.controller.onEditBlocked?.call('restrictedBlob');
              return;
            }
            if (node.type == 'horizontalRule') {
              scope.bottomToolbarMode.value = BottomToolbarMode.horizontalRule;
            } else if (node.type == 'blockquote') {
              scope.bottomToolbarMode.value = BottomToolbarMode.blockquote;
            } else if (node.type == 'table') {
              scope.bottomToolbarMode.value = BottomToolbarMode.tableSize;
            } else {
              scope.dispatch({'type': node.type, ...?node.attrs});
              scope.controller.scrollIntoView();
              switch (keyboardType) {
                case KeyboardType.software:
                  scope.requestFocus();
                case KeyboardType.hardware:
                  scope.bottomToolbarMode.value = BottomToolbarMode.hidden;
              }
            }
          },
          builder: (context, color, backgroundColor) {
            return Column(
              mainAxisAlignment: MainAxisAlignment.center,
              spacing: 12,
              children: [
                SvgImage('icons/${node.icon}', width: 28, height: 28, color: color),
                Text(node.label, style: TextStyle(fontSize: 15, color: color)),
              ],
            );
          },
        );
      }).toList(),
    );
  }
}
