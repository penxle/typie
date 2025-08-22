import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/editor_floating_fade.dart';

class EditorFloatingWidget extends HookWidget {
  const EditorFloatingWidget({
    required this.child,
    required this.onPositionChanged,
    this.initialRelativePosition,
    this.isExpanded = false,
    this.onTap,
    super.key,
  });

  final Widget child;
  final void Function(Offset relativePosition) onPositionChanged;
  final Offset? initialRelativePosition;
  final bool isExpanded;
  final VoidCallback? onTap;

  Offset _clampPosition(Offset position, Size widgetSize, Size containerSize) {
    final maxX = (containerSize.width - widgetSize.width).clamp(0.0, double.infinity);
    final maxY = (containerSize.height - widgetSize.height).clamp(0.0, double.infinity);

    return Offset(position.dx.clamp(0.0, maxX), position.dy.clamp(0.0, maxY));
  }

  Offset _toAbsolutePosition(Offset relativePos, Size containerSize) {
    return Offset(relativePos.dx * containerSize.width, relativePos.dy * containerSize.height);
  }

  Offset _toRelativePosition(Offset absolutePos, Size containerSize) {
    return Offset(absolutePos.dx / containerSize.width, absolutePos.dy / containerSize.height);
  }

  @override
  Widget build(BuildContext context) {
    MediaQuery.of(context);

    final widgetKey = useMemoized(GlobalKey.new);
    final widgetSize = useState<Size?>(null);

    const defaultRelativePosition = Offset(0.05, 0.05);

    final screenSize = MediaQuery.of(context).size;
    final relativePos = initialRelativePosition ?? defaultRelativePosition;
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

    // NOTE: 초기 상대 위치에서 절대 위치 계산
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        final editorContainer = context.findAncestorRenderObjectOfType<RenderBox>();
        if (editorContainer != null && editorContainer.hasSize) {
          final relativePos = initialRelativePosition ?? defaultRelativePosition;
          currentRelativePosition.value = relativePos;

          final absolutePos = _toAbsolutePosition(relativePos, editorContainer.size);
          position.value = _clampPosition(absolutePos, widgetSize.value ?? const Size(120, 40), editorContainer.size);

          previousEditorSize.value = editorContainer.size;
        }
      });
      return null;
    }, []);

    // NOTE: 화면 크기 변경 시 위치 재계산
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final renderBox = widgetKey.currentContext?.findRenderObject() as RenderBox?;
      if (renderBox != null && renderBox.hasSize) {
        widgetSize.value = renderBox.size;
      }

      // NOTE: 에디터 컨테이너 크기 변경 감지 및 위치 재계산
      if (!isDragging.value && previousEditorSize.value != null) {
        final editorContainer = context.findAncestorRenderObjectOfType<RenderBox>();
        if (editorContainer != null && editorContainer.hasSize) {
          final currentEditorSize = editorContainer.size;

          // NOTE: 크기가 변경되었을 때만 위치 재계산
          if (previousEditorSize.value != null && previousEditorSize.value != currentEditorSize) {
            final relativePos = currentRelativePosition.value ?? initialRelativePosition ?? defaultRelativePosition;

            final absolutePos = _toAbsolutePosition(relativePos, currentEditorSize);
            position.value = _clampPosition(absolutePos, widgetSize.value ?? const Size(120, 40), currentEditorSize);
          }

          previousEditorSize.value = currentEditorSize;
        }
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
