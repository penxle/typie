import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/controller/clipboard.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/scroll.dart';

class SelectionContextMenu extends StatelessWidget {
  const SelectionContextMenu({required this.clipboard, required this.onDismiss, super.key});

  final EditorClipboard clipboard;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);

    return ListenableBuilder(
      listenable: Listenable.merge([scope.verticalScrollController, scope.horizontalScrollController]),
      builder: (context, _) {
        final anchor = _computeMenuAnchor(scope);
        if (anchor == null) {
          return const SizedBox.shrink();
        }

        return Positioned.fill(
          child: CustomSingleChildLayout(
            delegate: _MenuPositionDelegate(centerX: anchor.centerX, above: anchor.above, below: anchor.below),
            child: TweenAnimationBuilder<double>(
              tween: Tween(begin: 0, end: 1),
              duration: const Duration(milliseconds: 150),
              curve: Curves.easeOutCubic,
              builder: (context, value, child) {
                return Opacity(
                  opacity: value,
                  child: Transform.scale(scale: 0.8 + 0.2 * value, child: child),
                );
              },
              child: _MenuBubble(clipboard: clipboard, onDismiss: onDismiss),
            ),
          ),
        );
      },
    );
  }

  ({double centerX, double above, double below})? _computeMenuAnchor(ContentScope scope) {
    final geo = scope.geometry;
    final offsets = geo.computeCumulativePageOffsets();
    final vController = scope.verticalScrollController;
    final hController = scope.horizontalScrollController;

    if (!vController.hasSingleClient) {
      return null;
    }

    final scrollOffset = vController.offset;
    final hScrollOffset = hController.hasSingleClient ? hController.offset : 0.0;

    final state = scope.controller.state;
    final fromHandle = state.selection?.fromBounds;
    final toHandle = state.selection?.toBounds;
    final cursor = state.cursor;

    double topY;
    double bottomY;
    double centerX;

    if (fromHandle != null && toHandle != null) {
      final fromPageTop = geo.titleAreaHeight + offsets[fromHandle.pageIdx];
      final fromScreenY = fromPageTop + fromHandle.y - scrollOffset;
      final fromScreenX = geo.horizontalPadding + fromHandle.x - hScrollOffset;

      final toPageTop = geo.titleAreaHeight + offsets[toHandle.pageIdx];
      final toScreenY = toPageTop + toHandle.y + toHandle.height - scrollOffset;
      final toScreenX = geo.horizontalPadding + toHandle.x - hScrollOffset;

      topY = fromScreenY;
      bottomY = toScreenY;
      centerX = (fromScreenX + toScreenX) / 2;
    } else if (cursor != null) {
      final cursorPageTop = geo.titleAreaHeight + offsets[cursor.pageIdx];
      final cursorScreenY = cursorPageTop + cursor.y - scrollOffset;
      final cursorScreenX = geo.horizontalPadding + cursor.x - hScrollOffset;

      topY = cursorScreenY;
      bottomY = cursorScreenY + cursor.height;
      centerX = cursorScreenX;
    } else {
      return null;
    }

    const gap = 24.0;
    final above = topY - gap;
    final below = bottomY + gap;

    return (centerX: centerX, above: above, below: below);
  }
}

class _MenuPositionDelegate extends SingleChildLayoutDelegate {
  const _MenuPositionDelegate({required this.centerX, required this.above, required this.below});

  final double centerX;
  final double above;
  final double below;

  static const _padding = 4.0;

  @override
  Size getSize(BoxConstraints constraints) => constraints.biggest;

  @override
  BoxConstraints getConstraintsForChild(BoxConstraints constraints) {
    return BoxConstraints.loose(constraints.biggest);
  }

  @override
  Offset getPositionForChild(Size size, Size childSize) {
    final showBelow = above < childSize.height;

    var x = centerX - childSize.width / 2;
    x = x.clamp(_padding, size.width - childSize.width - _padding);

    final y = showBelow ? below : above - childSize.height;

    return Offset(x, y);
  }

  @override
  bool shouldRelayout(_MenuPositionDelegate oldDelegate) {
    return centerX != oldDelegate.centerX || above != oldDelegate.above || below != oldDelegate.below;
  }
}

class _MenuBubble extends HookWidget {
  const _MenuBubble({required this.clipboard, required this.onDismiss});

  final EditorClipboard clipboard;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final colors = context.colors;
    final selection = scope.controller.state.selection;
    final hasSelection = selection?.collapsed == false;
    final isExpanded = useState(false);

    final controller = useAnimationController(duration: const Duration(milliseconds: 150));
    final curve = useMemoized(
      () => CurvedAnimation(parent: controller, curve: Curves.easeOut, reverseCurve: Curves.easeIn),
      [controller],
    );

    useEffect(() {
      if (isExpanded.value) {
        unawaited(controller.forward());
      } else {
        unawaited(controller.reverse());
      }
      return null;
    }, [isExpanded.value]);

    final mainOpacity = Tween<double>(begin: 1, end: 0);
    final mainOffset = Tween<double>(begin: 0, end: -10);
    final subOpacity = Tween<double>(begin: 0, end: 1);
    final subOffset = Tween<double>(begin: 10, end: 0);

    final mainMenu = Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (hasSelection) ...[
          _MenuButton(
            label: '복사',
            onTap: () async {
              await clipboard.copy(scope.editor);
              onDismiss();
            },
          ),
          _MenuButton(
            label: '잘라내기',
            onTap: () async {
              await clipboard.cut(scope.editor, scope.editor.dispatch);
              onDismiss();
            },
          ),
        ],
        _MenuButton(
          label: '붙여넣기',
          onTap: () {
            unawaited(
              EditorClipboard().getPastePayload().then((payload) {
                scope.editor.dispatch(payload);
                scope.controller.scrollIntoView();
                onDismiss();
              }),
            );
          },
        ),
        if (selection?.canExpand ?? true) _MenuButton(label: '선택 확장', onTap: () => isExpanded.value = true),
      ],
    );

    final subMenu = Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: () => isExpanded.value = false,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
            child: Icon(LucideLightIcons.chevron_left, size: 14, color: colors.textDefault),
          ),
        ),
        if (selection?.canExpandWord ?? true)
          _MenuButton(
            label: '단어',
            onTap: () {
              scope.editor.dispatch({'type': 'selectWord'});
              scope.controller.scrollIntoView();
              isExpanded.value = false;
            },
          ),
        if (selection?.canExpandSentence ?? true)
          _MenuButton(
            label: '문장',
            onTap: () {
              scope.editor.dispatch({'type': 'selectSentence'});
              scope.controller.scrollIntoView();
              isExpanded.value = false;
            },
          ),
        if (selection?.canExpandParagraph ?? true)
          _MenuButton(
            label: '문단',
            onTap: () {
              scope.editor.dispatch({'type': 'selectParagraph'});
              scope.controller.scrollIntoView();
              isExpanded.value = false;
            },
          ),
        if (selection?.canExpandAll ?? true)
          _MenuButton(
            label: '전체',
            onTap: () {
              scope.editor.dispatch({'type': 'selectAll'});
              isExpanded.value = false;
            },
          ),
      ],
    );

    return AnimatedSize(
      duration: const Duration(milliseconds: 150),
      curve: Curves.easeOut,
      clipBehavior: Clip.none,
      child: Container(
        decoration: BoxDecoration(
          color: colors.surfaceDefault,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: colors.borderDefault),
          boxShadow: const [BoxShadow(color: Color(0x1A000000), blurRadius: 8, offset: Offset(0, 2))],
        ),
        clipBehavior: Clip.hardEdge,
        child: AnimatedBuilder(
          animation: controller,
          builder: (context, _) {
            final target = isExpanded.value ? subMenu : mainMenu;
            final outgoing = isExpanded.value ? mainMenu : subMenu;
            final outgoingOpacity = isExpanded.value ? mainOpacity : subOpacity;
            final outgoingOffset = isExpanded.value ? mainOffset : subOffset;
            final targetOpacity = isExpanded.value ? subOpacity : mainOpacity;
            final targetOffset = isExpanded.value ? subOffset : mainOffset;
            final outgoingVisible = !(isExpanded.value ? controller.isCompleted : controller.isDismissed);

            return Stack(
              alignment: Alignment.centerLeft,
              children: [
                // 대상 뷰: non-positioned → Stack 크기 결정, AnimatedSize가 부드럽게 전환
                Transform.translate(
                  offset: Offset(targetOffset.evaluate(curve), 0),
                  child: Opacity(opacity: targetOpacity.evaluate(curve), child: target),
                ),
                // 나가는 뷰: OverflowBox로 자연 크기 유지, Container clip으로 잘림
                if (outgoingVisible)
                  Positioned.fill(
                    child: OverflowBox(
                      maxWidth: double.infinity,
                      alignment: Alignment.centerLeft,
                      child: Transform.translate(
                        offset: Offset(outgoingOffset.evaluate(curve), 0),
                        child: Opacity(opacity: outgoingOpacity.evaluate(curve), child: outgoing),
                      ),
                    ),
                  ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _MenuButton extends StatelessWidget {
  const _MenuButton({required this.label, required this.onTap});

  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
        child: Text(label, style: TextStyle(fontSize: 14, color: colors.textDefault)),
      ),
    );
  }
}
