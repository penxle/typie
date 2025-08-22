import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/editor_floating_fade.dart';

class EditorFloatingWidget extends HookWidget {
  EditorFloatingWidget({
    required this.child,
    required this.onPositionChanged,
    this.initialRelativePosition,
    this.isExpanded = false,
    this.onTap,
    super.key,
  }) : assert(
         initialRelativePosition == null ||
             (initialRelativePosition.dx >= 0.0 &&
                 initialRelativePosition.dx <= 1.0 &&
                 initialRelativePosition.dy >= 0.0 &&
                 initialRelativePosition.dy <= 1.0),
         'initialRelativePosition must be within 0.0–1.0',
       );

  final Widget child;
  final void Function(Offset relativePosition) onPositionChanged;
  final Offset? initialRelativePosition;
  final bool isExpanded;
  final void Function(bool isFaded)? onTap;

  Offset _clampPosition(Offset position, Size widgetSize, Size containerSize) {
    final maxX = (containerSize.width - widgetSize.width).clamp(0.0, double.infinity);
    final maxY = (containerSize.height - widgetSize.height).clamp(0.0, double.infinity);

    return Offset(position.dx.clamp(0.0, maxX), position.dy.clamp(0.0, maxY));
  }

  Offset _toAbsolutePosition(Offset relativePos, Size containerSize) {
    return Offset(relativePos.dx * containerSize.width, relativePos.dy * containerSize.height);
  }

  Offset _toRelativePosition(Offset absolutePos, Size containerSize) {
    final w = containerSize.width;
    final h = containerSize.height;
    if (w == 0 || h == 0) {
      return Offset.zero;
    }
    return Offset(absolutePos.dx / w, absolutePos.dy / h);
  }

  @override
  Widget build(BuildContext context) {
    final widgetKey = useMemoized(GlobalKey.new);
    final widgetSize = useState<Size?>(null);

    final screenSize = MediaQuery.of(context).size;
    final relativePos = initialRelativePosition ?? const Offset(0.5, 0.5);
    final initialX = relativePos.dx * screenSize.width;
    final initialY = relativePos.dy * screenSize.height;

    final position = useState<Offset>(Offset(initialX, initialY));
    final isDragging = useState(false);
    final currentRelativePosition = useState<Offset?>(initialRelativePosition);
    final previousEditorSize = useState<Size?>(null);

    final originalPosition = useState<Offset?>(null);
    final hasDragged = useState(false);
    final wasExpanded = useState(false);

    final fadeController = useEditorFloatingFade();

    // NOTE: 확장 상태 변화 감지
    useEffect(() {
      if (isExpanded && !wasExpanded.value) {
        originalPosition.value = position.value;
        hasDragged.value = false;
        wasExpanded.value = true;

        WidgetsBinding.instance.addPostFrameCallback((_) {
          final renderBox = widgetKey.currentContext?.findRenderObject() as RenderBox?;
          final editorContainer = context.findAncestorRenderObjectOfType<RenderBox>();

          if (renderBox != null && editorContainer != null && renderBox.hasSize && editorContainer.hasSize) {
            final adjustedPosition = _clampPosition(position.value, renderBox.size, editorContainer.size);

            if (adjustedPosition != position.value) {
              position.value = adjustedPosition;
            }
          }
        });
      } else if (!isExpanded && wasExpanded.value) {
        if (!hasDragged.value && originalPosition.value != null) {
          position.value = originalPosition.value!;
        }
        originalPosition.value = null;
        hasDragged.value = false;
        wasExpanded.value = false;
      }
      return null;
    }, [isExpanded]);

    // NOTE: 초기 상대 위치에서 절대 위치 계산 (위젯 크기 측정 후)
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        final renderBox = widgetKey.currentContext?.findRenderObject() as RenderBox?;
        final editorContainer = context.findAncestorRenderObjectOfType<RenderBox>();

        if (renderBox != null && renderBox.hasSize && editorContainer != null && editorContainer.hasSize) {
          if (initialRelativePosition == null) {
            return;
          }

          final relativePos = initialRelativePosition!;
          currentRelativePosition.value = relativePos;

          final absolutePos = _toAbsolutePosition(relativePos, editorContainer.size);
          position.value = _clampPosition(absolutePos, renderBox.size, editorContainer.size);

          previousEditorSize.value = editorContainer.size;
          widgetSize.value = renderBox.size;
        }
      });
      return null;
    }, []);

    // NOTE: 화면 크기 변경 시 위치 재계산
    useEffect(() {
      void updateWidgetSizeAndPosition() {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          final renderBox = widgetKey.currentContext?.findRenderObject() as RenderBox?;
          if (renderBox != null && renderBox.hasSize && widgetSize.value != renderBox.size) {
            widgetSize.value = renderBox.size;
          }

          // NOTE: 에디터 컨테이너 크기 변경 감지 및 위치 재계산
          if (!isDragging.value && previousEditorSize.value != null) {
            final editorContainer = context.findAncestorRenderObjectOfType<RenderBox>();
            if (editorContainer != null && editorContainer.hasSize) {
              final currentEditorSize = editorContainer.size;

              // NOTE: 크기가 변경되었을 때만 위치 재계산
              if (previousEditorSize.value != currentEditorSize) {
                final relativePos = currentRelativePosition.value ?? initialRelativePosition;
                if (relativePos == null) {
                  return;
                }

                if (widgetSize.value != null) {
                  final absolutePos = _toAbsolutePosition(relativePos, currentEditorSize);
                  final newPosition = _clampPosition(absolutePos, widgetSize.value!, currentEditorSize);

                  if (position.value != newPosition) {
                    position.value = newPosition;
                  }
                }

                previousEditorSize.value = currentEditorSize;
              }
            }
          }
        });
      }

      updateWidgetSizeAndPosition();
      return null;
    });

    return AnimatedPositioned(
      duration: isDragging.value ? Duration.zero : const Duration(milliseconds: 250),
      curve: Curves.easeOutCubic,
      left: position.value.dx,
      top: position.value.dy,
      child: FadeTransition(
        opacity: fadeController.opacity,
        child: GestureDetector(
          onTap: () {
            final isFaded = fadeController.opacity.value < 1.0;
            fadeController.showImmediately();
            onTap?.call(isFaded);
          },
          onPanStart: (_) {
            isDragging.value = true;
            fadeController.showImmediately();
          },
          onPanUpdate: (details) {
            hasDragged.value = true;

            final editorContainer = context.findAncestorRenderObjectOfType<RenderBox>();

            if (widgetSize.value == null || editorContainer == null || !editorContainer.hasSize) {
              return;
            }

            final newPosition = Offset(position.value.dx + details.delta.dx, position.value.dy + details.delta.dy);

            final adjustedPosition = _clampPosition(newPosition, widgetSize.value!, editorContainer.size);

            position.value = adjustedPosition;
          },
          onPanEnd: (_) {
            isDragging.value = false;
            final editorContainer = context.findAncestorRenderObjectOfType<RenderBox>();
            if (editorContainer != null && editorContainer.hasSize) {
              final relativePos = _toRelativePosition(position.value, editorContainer.size);
              currentRelativePosition.value = relativePos;
              onPositionChanged(relativePos);
            }
          },
          onPanCancel: () {
            isDragging.value = false;
            final editorContainer = context.findAncestorRenderObjectOfType<RenderBox>();
            if (editorContainer != null && editorContainer.hasSize) {
              final relativePos = _toRelativePosition(position.value, editorContainer.size);
              currentRelativePosition.value = relativePos;
              onPositionChanged(relativePos);
            }
          },
          child: KeyedSubtree(key: widgetKey, child: child),
        ),
      ),
    );
  }
}
