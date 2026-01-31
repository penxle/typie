import 'package:flutter/material.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/controller/scroll_behavior.dart';
import 'package:typie/screens/native_editor/cursor.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';
import 'package:typie/screens/native_editor/view/page_item.dart';

class PageList extends StatelessWidget {
  const PageList({
    required this.editor,
    required this.layout,
    required this.cursor,
    required this.isFocused,
    required this.isSelecting,
    required this.renderVersion,
    required this.scrollController,
    required this.viewKeyboardHeight,
    required this.onOpenInput,
    required this.onSelectionStart,
    required this.onSelectionEnd,
    super.key,
  });

  final NativeEditor editor;
  final LayoutInfo layout;
  final CursorInfo? cursor;
  final bool isFocused;
  final bool isSelecting;
  final Object? renderVersion;
  final ScrollController scrollController;
  final double viewKeyboardHeight;
  final VoidCallback onOpenInput;
  final VoidCallback onSelectionStart;
  final VoidCallback onSelectionEnd;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onOpenInput,
      child: ListView.builder(
        controller: scrollController,
        padding: EdgeInsets.only(bottom: viewKeyboardHeight),
        itemCount: layout.pageCount,
        cacheExtent: 1000,
        physics: isSelecting ? const NeverScrollableScrollPhysics() : const AlwaysScrollableScrollPhysics(),
        itemBuilder: (context, index) {
          final isLast = index == layout.pageCount - 1;
          final gap = layout.isPaginated && !isLast ? pageGap : 0.0;
          final pageHeight = layout.pageHeights.elementAtOrNull(index);
          final pageCursor = cursor?.pageIdx == index ? cursor : null;

          return PageItem(
            key: ValueKey(index),
            pageIndex: index,
            editor: editor,
            renderVersion: renderVersion,
            bottomGap: gap,
            placeholderHeight: pageHeight,
            cursorInfo: pageCursor,
            isFocused: isFocused,
            onSelectionStart: onSelectionStart,
            onSelectionEnd: onSelectionEnd,
            onTap: onOpenInput,
          );
        },
      ),
    );
  }
}
