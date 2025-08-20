import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/editor_floating_fade.dart';

class EditorFloatingWidget extends HookWidget {
  const EditorFloatingWidget({
    required this.child,
    required this.storageKey,
    required this.onPositionChanged,
    this.initialOffset,
    this.isExpanded = false,
    this.onExpansionChanged,
    this.onTap,
    super.key,
  });

  final Widget child;
  final String storageKey;
  final void Function(Offset position) onPositionChanged;
  final Offset? initialOffset;
  final bool isExpanded;
  final VoidCallback? onExpansionChanged;
  final VoidCallback? onTap;

  Offset _adjustPositionWithinBounds({
    required Offset currentPosition,
    required Size widgetSize,
    required Size containerSize,
  }) {
    var newX = currentPosition.dx;
    var newY = currentPosition.dy;

    if (newX < 0) {
      newX = 0;
    } else if (newX + widgetSize.width > containerSize.width) {
      newX = containerSize.width - widgetSize.width;
    }

    if (newY < 0) {
      newY = 0;
    } else if (newY + widgetSize.height > containerSize.height) {
      newY = containerSize.height - widgetSize.height;
    }

    return Offset(newX, newY);
  }

  @override
  Widget build(BuildContext context) {
    // NOTE: 키보드 등 레이아웃 변화 감지
    MediaQuery.of(context);

    final widgetKey = useMemoized(GlobalKey.new);
    final widgetSize = useState<Size?>(null);
    final position = useState(initialOffset ?? const Offset(20, 20));
    final previousEditorSize = useState<Size?>(null);
    final isDragging = useState(false);

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
            final adjustedPosition = _adjustPositionWithinBounds(
              currentPosition: position.value,
              widgetSize: renderBox.size,
              containerSize: editorContainer.size,
            );

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

    WidgetsBinding.instance.addPostFrameCallback((_) {
      final renderBox = widgetKey.currentContext?.findRenderObject() as RenderBox?;
      if (renderBox != null && renderBox.hasSize) {
        widgetSize.value = renderBox.size;
      }

      final editorContainer = context.findAncestorRenderObjectOfType<RenderBox>();
      if (editorContainer != null && editorContainer.hasSize) {
        final currentEditorSize = editorContainer.size;
        final currentWidgetSize = widgetSize.value ?? const Size(120, 40);

        // NOTE: 부모 크기가 변경되었을 때 상대 위치 유지
        if (previousEditorSize.value != null && previousEditorSize.value != currentEditorSize) {
          final oldHeight = previousEditorSize.value!.height;
          final newHeight = currentEditorSize.height;

          if (oldHeight > 0 && newHeight > 0) {
            final yPercentage = position.value.dy / oldHeight;
            final newY = yPercentage * newHeight;

            position.value = _adjustPositionWithinBounds(
              currentPosition: Offset(position.value.dx, newY),
              widgetSize: currentWidgetSize,
              containerSize: currentEditorSize,
            );
          }
        } else if (previousEditorSize.value == null) {
          // NOTE: 초기 로드 시 위치 조정
          position.value = _adjustPositionWithinBounds(
            currentPosition: position.value,
            widgetSize: currentWidgetSize,
            containerSize: currentEditorSize,
          );
        }

        previousEditorSize.value = currentEditorSize;
      }
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
            fadeController.showImmediately();
            onTap?.call();
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

            final adjustedPosition = _adjustPositionWithinBounds(
              currentPosition: newPosition,
              widgetSize: widgetSize.value!,
              containerSize: editorContainer.size,
            );

            position.value = adjustedPosition;
          },
          onPanEnd: (_) {
            isDragging.value = false;
            onPositionChanged(position.value);
          },
          onPanCancel: () {
            isDragging.value = false;
            onPositionChanged(position.value);
          },
          child: KeyedSubtree(key: widgetKey, child: child),
        ),
      ),
    );
  }
}
