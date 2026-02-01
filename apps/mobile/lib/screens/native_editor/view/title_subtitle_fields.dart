import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';

class TitleSubtitleFields extends HookWidget {
  const TitleSubtitleFields({
    required this.title,
    required this.subtitle,
    required this.onTitleChanged,
    required this.onSubtitleChanged,
    required this.titleFocusNode,
    required this.subtitleFocusNode,
    required this.onEnterDocument,
    required this.pageWidth,
    super.key,
  });

  final String title;
  final String subtitle;
  final ValueChanged<String> onTitleChanged;
  final ValueChanged<String> onSubtitleChanged;
  final FocusNode titleFocusNode;
  final FocusNode subtitleFocusNode;
  final VoidCallback onEnterDocument;
  final double pageWidth;

  @override
  Widget build(BuildContext context) {
    final titleController = useTextEditingController(text: title);
    final subtitleController = useTextEditingController(text: subtitle);

    useEffect(() {
      if (titleController.text != title) {
        titleController.text = title;
      }
      return null;
    }, [title]);

    useEffect(() {
      if (subtitleController.text != subtitle) {
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
          subtitleFocusNode.unfocus();
          WidgetsBinding.instance.addPostFrameCallback((_) => onEnterDocument());
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
        padding: const EdgeInsets.only(top: 40, left: 20, right: 20),
        child: Column(
          children: [
            Focus(
              onKeyEvent: handleTitleKeyEvent,
              child: TextField(
                controller: titleController,
                focusNode: titleFocusNode,
                style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold, color: context.colors.textDefault),
                textAlign: TextAlign.center,
                decoration: InputDecoration(
                  hintText: '제목',
                  hintStyle: TextStyle(fontSize: 20, fontWeight: FontWeight.bold, color: context.colors.textDisabled),
                  border: InputBorder.none,
                  contentPadding: EdgeInsets.zero,
                  isDense: true,
                ),
                maxLength: 100,
                maxLines: null,
                minLines: 1,
                textInputAction: TextInputAction.next,
                buildCounter: (context, {required currentLength, required isFocused, required maxLength}) => null,
                onChanged: onTitleChanged,
                onSubmitted: (_) => subtitleFocusNode.requestFocus(),
              ),
            ),
            const SizedBox(height: 4),
            Focus(
              onKeyEvent: handleSubtitleKeyEvent,
              child: TextField(
                controller: subtitleController,
                focusNode: subtitleFocusNode,
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textDefault),
                textAlign: TextAlign.center,
                decoration: InputDecoration(
                  hintText: '부제목',
                  hintStyle: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textDisabled),
                  border: InputBorder.none,
                  contentPadding: EdgeInsets.zero,
                  isDense: true,
                ),
                maxLength: 100,
                maxLines: null,
                minLines: 1,
                textInputAction: TextInputAction.next,
                buildCounter: (context, {required currentLength, required isFocused, required maxLength}) => null,
                onChanged: onSubtitleChanged,
                onSubmitted: (_) {
                  subtitleFocusNode.unfocus();
                  WidgetsBinding.instance.addPostFrameCallback((_) => onEnterDocument());
                },
              ),
            ),
            const SizedBox(height: 40),
            Container(width: 120, height: 1, color: context.colors.borderDefault),
          ],
        ),
      ),
    );
  }
}
