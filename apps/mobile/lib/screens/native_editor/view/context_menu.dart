import 'package:flutter/widgets.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/native_editor/controller/clipboard.dart';
import 'package:typie/screens/native_editor/sheet/paste_option.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/service.dart';
import 'package:typie/services/preference.dart';

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

    if (!vController.hasClients) {
      return null;
    }

    final scrollOffset = vController.offset;
    final hScrollOffset = hController.hasClients ? hController.offset : 0.0;

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

class _MenuBubble extends StatelessWidget {
  const _MenuBubble({required this.clipboard, required this.onDismiss});

  final EditorClipboard clipboard;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final colors = context.colors;
    final hasSelection = scope.controller.state.selection?.collapsed == false;

    return Container(
      decoration: BoxDecoration(
        color: colors.surfaceDefault,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: colors.borderDefault),
        boxShadow: const [BoxShadow(color: Color(0x1A000000), blurRadius: 8, offset: Offset(0, 2))],
      ),
      child: Row(
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
            onTap: () async {
              final pref = serviceLocator<Pref>();
              final payload = await clipboard.getPastePayload();
              final html = payload['html'] as String?;

              if (html != null && pref.pasteMode == 'ask') {
                onDismiss();
                if (!context.mounted) {
                  return;
                }
                await context.showBottomSheet(
                  intercept: true,
                  child: PasteOptionBottomSheet(
                    onConfirm: (selectedMode) async {
                      scope.editor.dispatch({...payload, 'mode': selectedMode == 'text' ? 'text' : 'auto'});
                      scope.controller.scrollIntoView();
                    },
                  ),
                );
                return;
              }

              scope.editor.dispatch({...payload, 'mode': pref.pasteMode == 'text' ? 'text' : 'auto'});
              scope.controller.scrollIntoView();
              onDismiss();
            },
          ),
          _MenuButton(
            label: '전체 선택',
            onTap: () {
              scope.editor.dispatch({'type': 'selectAll'});
              scope.controller.scrollIntoView();
              onDismiss();
            },
          ),
        ],
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
