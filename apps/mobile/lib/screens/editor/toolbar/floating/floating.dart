import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/floating/blockquote.dart';
import 'package:typie/screens/editor/toolbar/floating/callout.dart';
import 'package:typie/screens/editor/toolbar/floating/code.dart';
import 'package:typie/screens/editor/toolbar/floating/embed.dart';
import 'package:typie/screens/editor/toolbar/floating/file.dart';
import 'package:typie/screens/editor/toolbar/floating/fold.dart';
import 'package:typie/screens/editor/toolbar/floating/horizontal_rule.dart';
import 'package:typie/screens/editor/toolbar/floating/html.dart';
import 'package:typie/screens/editor/toolbar/floating/image.dart';
import 'package:typie/screens/editor/toolbar/floating/list.dart';
import 'package:typie/screens/editor/toolbar/floating/table.dart';

class EditorFloatingToolbar extends HookWidget {
  const EditorFloatingToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.localProseMirrorState);

    if (proseMirrorState == null) {
      return const SizedBox.shrink();
    }

    if (proseMirrorState.isNodeActive('bullet_list') || proseMirrorState.isNodeActive('ordered_list')) {
      return const ListFloatingToolbar();
    }

    if (proseMirrorState.isNodeActive('blockquote')) {
      return const BlockquoteFloatingToolbar();
    }

    if (proseMirrorState.isNodeActive('callout')) {
      return const CalloutFloatingToolbar();
    }

    if (proseMirrorState.isNodeActive('fold')) {
      return const FoldFloatingToolbar();
    }

    if (proseMirrorState.isNodeActive('code_block')) {
      return const CodeFloatingToolbar();
    }

    if (proseMirrorState.isNodeActive('html_block')) {
      return const HtmlFloatingToolbar();
    }

    if (proseMirrorState.isNodeActive('table')) {
      return const TableFloatingToolbar();
    }

    return switch (proseMirrorState.currentNode?.type) {
      'horizontal_rule' => const HorizontalRuleFloatingToolbar(),
      'image' => const ImageFloatingToolbar(),
      'file' => const FileFloatingToolbar(),
      'embed' => const EmbedFloatingToolbar(),
      _ => const SizedBox.shrink(),
    };
  }
}
