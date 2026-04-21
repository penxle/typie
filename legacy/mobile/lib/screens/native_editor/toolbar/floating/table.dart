import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/table/models.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/base.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/floating.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';

const _inTableContext = 'in_table';
const _selectedTableContext = 'selected_table';
const _tableProportionEpsilon = 0.001;

class NativeEditorTableFloatingToolbar extends HookWidget {
  const NativeEditorTableFloatingToolbar({required this.table, super.key});

  final TableOverlayInfo table;

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final floatingContext = useValueListenable(scope.floatingContext);
    final floatingNodeId = useValueListenable(scope.floatingNodeId);

    final isTableSelected = floatingContext == _selectedTableContext && floatingNodeId == table.tableId;
    final isInTable = floatingContext == _inTableContext;

    if (!isTableSelected && !isInTable) {
      return const SizedBox.shrink();
    }

    return Row(
      spacing: 8,
      children: isTableSelected
          ? [
              FloatingToolbarButton(
                icon: LucideLightIcons.trash_2,
                onTap: () {
                  scope.dispatch({'type': 'deleteNode', 'nodeId': table.tableId});
                  scope.controller.scrollIntoView();
                },
              ),
              if (table.proportion < 1 - _tableProportionEpsilon)
                FloatingToolbarButton(
                  icon: _tableAlignIcon(table.align),
                  onTap: () {
                    scope.dispatch({
                      'type': 'setTableAlign',
                      'tableId': table.tableId,
                      'align': _nextTableAlign(table.align),
                    });
                    scope.controller.scrollIntoView();
                  },
                ),
              _TableBorderStyleButton(
                style: table.borderStyle,
                onTap: () {
                  scope.dispatch({
                    'type': 'setTableBorderStyle',
                    'tableId': table.tableId,
                    'style': _nextTableBorderStyle(table.borderStyle),
                  });
                  scope.controller.scrollIntoView();
                },
              ),
            ]
          : [
              FloatingToolbarButton(
                icon: LucideLightIcons.grip_vertical,
                onTap: () {
                  scope.dispatch({'type': 'selectTable', 'tableId': table.tableId});
                  scope.controller.scrollIntoView();
                },
              ),
            ],
    );
  }
}

String _nextTableAlign(String align) {
  return switch (align) {
    'left' => 'center',
    'center' => 'right',
    'right' => 'left',
    _ => 'left',
  };
}

String _nextTableBorderStyle(String style) {
  return switch (style) {
    'solid' => 'dashed',
    'dashed' => 'dotted',
    'dotted' => 'none',
    'none' => 'solid',
    _ => 'solid',
  };
}

IconData _tableAlignIcon(String align) {
  return switch (align) {
    'center' => LucideLightIcons.align_center,
    'right' => LucideLightIcons.align_right,
    _ => LucideLightIcons.align_left,
  };
}

class _TableBorderStyleButton extends StatelessWidget {
  const _TableBorderStyleButton({required this.style, required this.onTap});

  final String style;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ToolbarButton(
      onTap: onTap,
      builder: (context, color, backgroundColor) {
        return Container(
          decoration: BoxDecoration(
            color: backgroundColor,
            border: Border.all(color: color),
            borderRadius: BorderRadius.circular(999),
          ),
          padding: const EdgeInsets.all(8),
          child: SizedBox(
            width: 20,
            height: 20,
            child: _TableBorderStylePreview(style: style, color: color),
          ),
        );
      },
    );
  }
}

class _TableBorderStylePreview extends StatelessWidget {
  const _TableBorderStylePreview({required this.style, required this.color});

  final String style;
  final Color color;

  @override
  Widget build(BuildContext context) {
    if (style == 'none') {
      return Icon(LucideLightIcons.ban, size: 18, color: color);
    }

    return CustomPaint(
      painter: _TableBorderStylePreviewPainter(style: style, color: color),
    );
  }
}

class _TableBorderStylePreviewPainter extends CustomPainter {
  const _TableBorderStylePreviewPainter({required this.style, required this.color});

  final String style;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final strokeWidth = switch (style) {
      'solid' || 'dashed' => 1.2,
      'dotted' => 1.6,
      _ => 1.6,
    };

    final paint = Paint()
      ..color = color
      ..strokeWidth = strokeWidth
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.butt
      ..strokeJoin = StrokeJoin.round;

    const left = 3.0;
    const top = 3.0;
    final right = size.width - 3.0;
    final bottom = size.height - 3.0;

    if (style == 'solid') {
      canvas.drawRRect(
        RRect.fromRectAndRadius(Rect.fromLTRB(left, top, right, bottom), const Radius.circular(2)),
        paint..style = PaintingStyle.stroke,
      );
      return;
    }

    _drawStyledLine(canvas, paint, const Offset(left, top), Offset(right, top));
    _drawStyledLine(canvas, paint, Offset(right, top), Offset(right, bottom));
    _drawStyledLine(canvas, paint, Offset(right, bottom), Offset(left, bottom));
    _drawStyledLine(canvas, paint, Offset(left, bottom), const Offset(left, top));
  }

  void _drawStyledLine(Canvas canvas, Paint paint, Offset start, Offset end) {
    final isHorizontal = start.dy == end.dy;
    final length = isHorizontal ? (end.dx - start.dx).abs() : (end.dy - start.dy).abs();
    if (length <= 0) {
      return;
    }

    if (style == 'dotted') {
      const step = 3.6;
      final dotPaint = Paint()
        ..color = color
        ..style = PaintingStyle.fill;
      final radius = paint.strokeWidth * 0.5;
      for (var d = 0.0; d < length; d += step) {
        final point = isHorizontal
            ? Offset(start.dx + (end.dx >= start.dx ? d : -d), start.dy)
            : Offset(start.dx, start.dy + (end.dy >= start.dy ? d : -d));
        canvas.drawCircle(point, radius, dotPaint);
      }
      return;
    }

    const dash = 3.6;
    const gap = 2.6;
    var d = 0.0;
    while (d < length) {
      final next = (d + dash).clamp(0.0, length);
      final from = isHorizontal
          ? Offset(start.dx + (end.dx >= start.dx ? d : -d), start.dy)
          : Offset(start.dx, start.dy + (end.dy >= start.dy ? d : -d));
      final to = isHorizontal
          ? Offset(start.dx + (end.dx >= start.dx ? next : -next), start.dy)
          : Offset(start.dx, start.dy + (end.dy >= start.dy ? next : -next));
      canvas.drawLine(from, to, paint);
      d += dash + gap;
    }
  }

  @override
  bool shouldRepaint(covariant _TableBorderStylePreviewPainter oldDelegate) {
    return oldDelegate.style != style || oldDelegate.color != color;
  }
}
