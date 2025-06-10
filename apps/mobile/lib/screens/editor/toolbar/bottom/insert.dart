import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/base.dart';
import 'package:typie/screens/editor/values.dart';
import 'package:typie/widgets/svg_image.dart';

class _Node {
  const _Node({required this.icon, required this.label, required this.type, this.activeNodeTypes, this.attrs});

  final String icon;
  final String label;
  final String type;
  final List<String>? activeNodeTypes;
  final Map<String, dynamic>? attrs;
}

class InsertBottomToolbar extends HookWidget {
  const InsertBottomToolbar({super.key});

  static final _nodes = [
    const _Node(icon: 'image', label: '이미지', type: 'image'),
    const _Node(icon: 'paperclip', label: '파일', type: 'file'),
    const _Node(icon: 'file-up', label: '임베드', type: 'embed'),
    _Node(
      icon: 'horizontal-rule',
      label: '구분선',
      type: 'horizontal_rule',
      attrs: {'type': editorDefaultValues['horizontalRule']},
    ),
    _Node(icon: 'quote', label: '인용구', type: 'blockquote', attrs: {'type': editorDefaultValues['blockquote']}),
    const _Node(icon: 'gallery-vertical-end', label: '콜아웃', type: 'callout'),
    const _Node(icon: 'chevrons-down-up', label: '폴드', type: 'fold'),
    const _Node(icon: 'table', label: '표', type: 'table'),
    const _Node(icon: 'list', label: '목록', type: 'bullet_list', activeNodeTypes: ['bullet_list', 'ordered_list']),
    const _Node(icon: 'code', label: '코드', type: 'code_block'),
    const _Node(icon: 'code-xml', label: 'HTML', type: 'html_block'),
  ];

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return GridView.extent(
      maxCrossAxisExtent: 96,
      padding: Pad(all: 16, bottom: MediaQuery.paddingOf(context).bottom),
      mainAxisSpacing: 16,
      crossAxisSpacing: 16,
      children: _nodes.map((node) {
        final activeNodeTypes = node.activeNodeTypes ?? [node.type];
        final isActive = activeNodeTypes.any((nodeType) => proseMirrorState?.isNodeActive(nodeType) ?? false);

        return ToolbarButton(
          isActive: isActive,
          onTap: () async {
            await scope.command(node.type, attrs: node.attrs);
            await webViewController?.requestFocus();
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
