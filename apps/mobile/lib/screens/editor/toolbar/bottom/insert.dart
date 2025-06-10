import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/toolbox.dart';
import 'package:typie/screens/editor/values.dart';

class InsertBottomToolbar extends HookWidget {
  const InsertBottomToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return GridView.extent(
      maxCrossAxisExtent: 96,
      padding: const Pad(all: 16),
      mainAxisSpacing: 16,
      crossAxisSpacing: 16,
      children: [
        ToolboxToolbarButton(
          icon: 'image',
          label: '이미지',
          isActive: proseMirrorState?.isNodeActive('image') ?? false,
          onTap: () async {
            await scope.command('image');
            await webViewController?.requestFocus();
          },
        ),
        ToolboxToolbarButton(
          icon: 'paperclip',
          label: '파일',
          isActive: proseMirrorState?.isNodeActive('file') ?? false,
          onTap: () async {
            await scope.command('file');
            await webViewController?.requestFocus();
          },
        ),
        ToolboxToolbarButton(
          icon: 'file-up',
          label: '임베드',
          isActive: proseMirrorState?.isNodeActive('embed') ?? false,
          onTap: () async {
            await scope.command('embed');
            await webViewController?.requestFocus();
          },
        ),
        ToolboxToolbarButton(
          icon: 'horizontal-rule',
          label: '구분선',
          isActive: proseMirrorState?.isNodeActive('horizontal_rule') ?? false,
          onTap: () async {
            await scope.command('horizontal_rule', attrs: {'horizontalRule': editorDefaultValues['horizontalRule']});
            await webViewController?.requestFocus();
          },
        ),
        ToolboxToolbarButton(
          icon: 'quote',
          label: '인용구',
          isActive: proseMirrorState?.isNodeActive('blockquote') ?? false,
          onTap: () async {
            await scope.command('blockquote', attrs: {'blockquote': editorDefaultValues['blockquote']});
            await webViewController?.requestFocus();
          },
        ),
        ToolboxToolbarButton(
          icon: 'gallery-vertical-end',
          label: '콜아웃',
          isActive: proseMirrorState?.isNodeActive('callout') ?? false,
          onTap: () async {
            await scope.command('callout');
            await webViewController?.requestFocus();
          },
        ),
        ToolboxToolbarButton(
          icon: 'chevrons-down-up',
          label: '폴드',
          isActive: proseMirrorState?.isNodeActive('fold') ?? false,
          onTap: () async {
            await scope.command('fold');
            await webViewController?.requestFocus();
          },
        ),
        ToolboxToolbarButton(
          icon: 'table',
          label: '표',
          isActive: proseMirrorState?.isNodeActive('table') ?? false,
          onTap: () async {
            await scope.command('table');
            await webViewController?.requestFocus();
          },
        ),
        ToolboxToolbarButton(
          icon: 'list',
          label: '목록',
          isActive:
              (proseMirrorState?.isNodeActive('bullet_list') ?? false) ||
              (proseMirrorState?.isNodeActive('ordered_list') ?? false),
          onTap: () async {
            await scope.command('bullet_list');
            await webViewController?.requestFocus();
          },
        ),
        ToolboxToolbarButton(
          icon: 'code',
          label: '코드',
          isActive: proseMirrorState?.isNodeActive('code_block') ?? false,
          onTap: () async {
            await scope.command('code_block');
            await webViewController?.requestFocus();
          },
        ),
        ToolboxToolbarButton(
          icon: 'code-xml',
          label: 'HTML',
          isActive: proseMirrorState?.isNodeActive('html_block') ?? false,
          onTap: () async {
            await scope.command('html_block');
            await webViewController?.requestFocus();
          },
        ),
      ],
    );
  }
}
