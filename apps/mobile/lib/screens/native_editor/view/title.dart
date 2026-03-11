import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/scope.dart';

class TitleFields extends HookWidget {
  const TitleFields({
    required this.title,
    required this.subtitle,
    required this.onEnterDocument,
    required this.pageWidth,
    this.onFieldTap,
    super.key,
  });

  final String title;
  final String subtitle;
  final VoidCallback onEnterDocument;
  final double pageWidth;
  final VoidCallback? onFieldTap;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final onTitleChanged = scope.onTitleChanged;
    final onSubtitleChanged = scope.onSubtitleChanged;
    final titleFocusNode = scope.titleFocusNode;
    final subtitleFocusNode = scope.subtitleFocusNode;
    final isPaginatedLayout = scope.controller.state.layout is PaginatedLayout;

    final titleController = useTextEditingController(text: title);
    final subtitleController = useTextEditingController(text: subtitle);
    final topPadding = ContentGeometry.pagePadding + MediaQuery.paddingOf(context).top + 12;

    useEffect(() {
      if (titleController.text != title && !titleFocusNode.hasFocus) {
        titleController.text = title;
      }
      return null;
    }, [title]);

    useEffect(() {
      if (subtitleController.text != subtitle && !subtitleFocusNode.hasFocus) {
        subtitleController.text = subtitle;
      }
      return null;
    }, [subtitle]);

    KeyEventResult handleTitleKeyEvent(FocusNode node, KeyEvent event) {
      if (event is KeyDownEvent) {
        final isShiftPressed = HardwareKeyboard.instance.isShiftPressed;
        if (event.logicalKey == LogicalKeyboardKey.tab && isShiftPressed) {
          return KeyEventResult.handled;
        }
        if (event.logicalKey == LogicalKeyboardKey.arrowDown ||
            (event.logicalKey == LogicalKeyboardKey.tab && !isShiftPressed)) {
          subtitleFocusNode.requestFocus();
          return KeyEventResult.handled;
        }
      }
      return KeyEventResult.ignored;
    }

    KeyEventResult handleSubtitleKeyEvent(FocusNode node, KeyEvent event) {
      if (event is KeyDownEvent) {
        final isShiftPressed = HardwareKeyboard.instance.isShiftPressed;
        if (event.logicalKey == LogicalKeyboardKey.arrowUp ||
            (event.logicalKey == LogicalKeyboardKey.tab && isShiftPressed)) {
          titleFocusNode.requestFocus();
          return KeyEventResult.handled;
        } else if (event.logicalKey == LogicalKeyboardKey.arrowDown ||
            (event.logicalKey == LogicalKeyboardKey.tab && !isShiftPressed)) {
          onEnterDocument();
          return KeyEventResult.handled;
        } else if (event.logicalKey == LogicalKeyboardKey.backspace && subtitleController.text.isEmpty) {
          titleFocusNode.requestFocus();
          return KeyEventResult.handled;
        }
      }
      return KeyEventResult.ignored;
    }

    return FocusTraversalGroup(
      descendantsAreTraversable: false,
      child: Container(
        width: pageWidth,
        padding: EdgeInsets.only(top: topPadding, left: 20, right: 20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            GestureDetector(
              onTapDown: (_) {
                onFieldTap?.call();
                titleFocusNode.requestFocus();
              },
              child: Focus(
                onKeyEvent: handleTitleKeyEvent,
                child: TextField(
                  controller: titleController,
                  focusNode: titleFocusNode,
                  style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold, color: context.colors.textDefault),
                  textAlign: TextAlign.left,
                  decoration: InputDecoration(
                    hintText: '제목',
                    hintStyle: TextStyle(fontSize: 20, fontWeight: FontWeight.bold, color: context.colors.textDisabled),
                    border: InputBorder.none,
                    contentPadding: EdgeInsets.zero,
                    isDense: true,
                  ),
                  autocorrect: false,
                  maxLength: 100,
                  maxLines: null,
                  minLines: 1,
                  textInputAction: TextInputAction.next,
                  buildCounter: (context, {required currentLength, required isFocused, required maxLength}) => null,
                  onChanged: onTitleChanged,
                  onSubmitted: (_) => subtitleFocusNode.requestFocus(),
                ),
              ),
            ),
            const SizedBox(height: 4),
            GestureDetector(
              onTapDown: (_) {
                onFieldTap?.call();
                subtitleFocusNode.requestFocus();
              },
              child: Focus(
                onKeyEvent: handleSubtitleKeyEvent,
                child: TextField(
                  controller: subtitleController,
                  focusNode: subtitleFocusNode,
                  style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textDefault),
                  textAlign: TextAlign.left,
                  decoration: InputDecoration(
                    hintText: '부제목',
                    hintStyle: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textDisabled),
                    border: InputBorder.none,
                    contentPadding: EdgeInsets.zero,
                    isDense: true,
                  ),
                  autocorrect: false,
                  maxLength: 100,
                  maxLines: null,
                  minLines: 1,
                  textInputAction: TextInputAction.next,
                  buildCounter: (context, {required currentLength, required isFocused, required maxLength}) => null,
                  onChanged: onSubtitleChanged,
                  onSubmitted: (_) => onEnterDocument(),
                ),
              ),
            ),
            const SizedBox(height: ContentGeometry.pagePadding),
            if (!isPaginatedLayout) ...[
              Container(width: 120, height: 1, color: context.colors.borderDefault),
              const SizedBox(height: ContentGeometry.pagePadding),
            ],
          ],
        ),
      ),
    );
  }
}
