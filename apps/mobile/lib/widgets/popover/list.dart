import 'dart:async';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/styles/semantic_colors.dart';
import 'package:typie/widgets/popover/popover.dart';

const _selectionArmDelay = Duration(milliseconds: 150);

class PopoverListItem {
  const PopoverListItem({required this.child, required this.onSelected, this.key});

  final Key? key;
  final Widget child;
  final VoidCallback onSelected;
}

class PopoverList extends HookWidget {
  const PopoverList({
    required this.items,
    this.indicatorColor,
    this.itemBorderRadius = const BorderRadius.all(Radius.circular(Popover.expandedRadius - Popover.panePadding)),
    super.key,
  });

  final List<PopoverListItem> items;
  final Color? indicatorColor;
  final BorderRadius itemBorderRadius;

  @override
  Widget build(BuildContext context) {
    final listKey = useMemoized(GlobalKey.new);
    final itemKeys = useMemoized(() => List.generate(items.length, (_) => GlobalKey()), [items.length]);
    final itemKeysRef = useRef(itemKeys);
    final trackingSourceRef = useRef<_TrackingSource?>(null);
    final trackedPointerRef = useRef<int?>(null);
    final selectionArmTimerRef = useRef<Timer?>(null);
    final latestTrackedEventRef = useRef<PointerEvent?>(null);
    final isSelectionArmedRef = useRef(false);
    final activeIndex = useState<int?>(null);
    final activeRect = useState<Rect?>(null);
    final setActiveIndexRef = useRef<void Function(int?)>((_) {});
    final resetTrackingRef = useRef<VoidCallback>(() {});
    final handleTrackedEventRef = useRef<void Function(PointerEvent)>((_) {});
    itemKeysRef.value = itemKeys;

    int? indexAtPosition(Offset position) {
      for (var index = 0; index < itemKeysRef.value.length; index++) {
        final renderObject = itemKeysRef.value[index].currentContext?.findRenderObject();
        if (renderObject is! RenderBox || !renderObject.hasSize) {
          continue;
        }

        final rect = renderObject.localToGlobal(Offset.zero) & renderObject.size;
        if (rect.contains(position)) {
          return index;
        }
      }

      return null;
    }

    Rect? rectForIndex(int index) {
      final listRenderObject = listKey.currentContext?.findRenderObject();
      final itemRenderObject = itemKeysRef.value[index].currentContext?.findRenderObject();
      if (listRenderObject is! RenderBox || !listRenderObject.hasSize) {
        return null;
      }
      if (itemRenderObject is! RenderBox || !itemRenderObject.hasSize) {
        return null;
      }

      final itemTopLeft = itemRenderObject.localToGlobal(Offset.zero, ancestor: listRenderObject);
      final itemRect = itemTopLeft & itemRenderObject.size;
      return Rect.fromLTWH(itemRect.left, itemRect.top, itemRect.width, itemRect.height);
    }

    void setActiveIndex(int? index) {
      final nextRect = index == null ? null : rectForIndex(index);
      if (index == activeIndex.value && nextRect == activeRect.value) {
        return;
      }

      activeIndex.value = index;
      activeRect.value = nextRect;
    }

    setActiveIndexRef.value = setActiveIndex;

    final handleLocalPointerEvent = useMemoized<PointerRoute>(() {
      return (event) {
        if (trackingSourceRef.value != _TrackingSource.local || trackedPointerRef.value != event.pointer) {
          return;
        }

        handleTrackedEventRef.value(event);
      };
    });

    void resetTracking() {
      if (trackingSourceRef.value == _TrackingSource.local) {
        GestureBinding.instance.pointerRouter.removeGlobalRoute(handleLocalPointerEvent);
      }

      selectionArmTimerRef.value?.cancel();
      selectionArmTimerRef.value = null;
      trackingSourceRef.value = null;
      trackedPointerRef.value = null;
      latestTrackedEventRef.value = null;
      isSelectionArmedRef.value = false;
      setActiveIndex(null);
    }

    resetTrackingRef.value = resetTracking;

    void triggerHighlightHaptic() {
      unawaited(HapticFeedback.selectionClick());
    }

    void handleTrackedEvent(PointerEvent event) {
      if (event is PointerDownEvent || event is PointerMoveEvent) {
        latestTrackedEventRef.value = event;
        if (!isSelectionArmedRef.value) {
          return;
        }
        final nextIndex = indexAtPosition(event.position);
        final shouldTriggerHaptic = isSelectionArmedRef.value && nextIndex != null && nextIndex != activeIndex.value;
        setActiveIndex(nextIndex);
        if (shouldTriggerHaptic) {
          triggerHighlightHaptic();
        }
        return;
      }

      if (event is PointerUpEvent) {
        final selectedIndex = indexAtPosition(event.position);
        final onSelected = switch ((trackingSourceRef.value, isSelectionArmedRef.value, selectedIndex)) {
          (_TrackingSource.local, _, final int index) => items[index].onSelected,
          (_, true, final int index) => items[index].onSelected,
          _ => null,
        };

        resetTrackingRef.value();
        onSelected?.call();
        return;
      }

      if (event is PointerCancelEvent) {
        resetTrackingRef.value();
      }
    }

    handleTrackedEventRef.value = handleTrackedEvent;

    void handleLocalPointerDown(PointerDownEvent event) {
      if (trackingSourceRef.value != null) {
        return;
      }

      trackingSourceRef.value = _TrackingSource.local;
      trackedPointerRef.value = event.pointer;
      latestTrackedEventRef.value = event;
      isSelectionArmedRef.value = false;
      selectionArmTimerRef.value?.cancel();
      selectionArmTimerRef.value = Timer(_selectionArmDelay, () {
        if (trackingSourceRef.value != _TrackingSource.local || trackedPointerRef.value != event.pointer) {
          return;
        }
        isSelectionArmedRef.value = true;
        final latestEvent = latestTrackedEventRef.value;
        final nextIndex = latestEvent == null ? null : indexAtPosition(latestEvent.position);
        final shouldTriggerHaptic = nextIndex != null && nextIndex != activeIndex.value;
        setActiveIndex(nextIndex);
        if (shouldTriggerHaptic) {
          triggerHighlightHaptic();
        }
      });
      GestureBinding.instance.pointerRouter.addGlobalRoute(handleLocalPointerEvent);
      handleTrackedEvent(event);
    }

    final scopePointerEvents = PopoverPointerScope.maybeOf(context);
    useEffect(() {
      void handleScopePointerEvent() {
        final rawScopeState = scopePointerEvents?.value;
        if (trackingSourceRef.value == _TrackingSource.local) {
          return;
        }

        if (rawScopeState == null) {
          if (trackingSourceRef.value == _TrackingSource.scope) {
            resetTrackingRef.value();
          }
          return;
        }

        final scopeState = switch (rawScopeState) {
          final PopoverPointerState state => state,
          final PointerEvent event => PopoverPointerState(event: event, isSelectionArmed: false),
          _ => null,
        };
        if (scopeState == null) {
          if (trackingSourceRef.value == _TrackingSource.scope) {
            resetTrackingRef.value();
          }
          return;
        }

        final event = scopeState.event;

        if (trackingSourceRef.value == null) {
          trackingSourceRef.value = _TrackingSource.scope;
          trackedPointerRef.value = event.pointer;
          isSelectionArmedRef.value = false;
        }

        if (trackingSourceRef.value != _TrackingSource.scope || trackedPointerRef.value != event.pointer) {
          return;
        }

        isSelectionArmedRef.value = scopeState.isSelectionArmed;
        if (!isSelectionArmedRef.value) {
          setActiveIndexRef.value(null);
          return;
        }

        handleTrackedEventRef.value(event);
      }

      scopePointerEvents?.addListener(handleScopePointerEvent);
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (context.mounted) {
          handleScopePointerEvent();
        }
      });

      return () {
        scopePointerEvents?.removeListener(handleScopePointerEvent);
      };
    }, [scopePointerEvents]);

    useEffect(() {
      return () {
        selectionArmTimerRef.value?.cancel();
        if (trackingSourceRef.value == _TrackingSource.local) {
          GestureBinding.instance.pointerRouter.removeGlobalRoute(handleLocalPointerEvent);
        }
      };
    }, [handleLocalPointerEvent]);

    final semanticColors = Theme.of(context).extension<SemanticColors>();
    final resolvedIndicatorColor =
        indicatorColor ?? semanticColors?.surfaceMuted ?? Theme.of(context).colorScheme.surfaceContainerHighest;

    return Stack(
      key: listKey,
      children: [
        if (activeRect.value != null)
          AnimatedPositioned(
            duration: const Duration(milliseconds: 140),
            curve: Curves.easeOutCubic,
            left: activeRect.value!.left,
            top: activeRect.value!.top,
            width: activeRect.value!.width,
            height: activeRect.value!.height,
            child: IgnorePointer(
              child: DecoratedBox(
                decoration: ShapeDecoration(
                  color: resolvedIndicatorColor,
                  shape: RoundedSuperellipseBorder(borderRadius: itemBorderRadius),
                ),
              ),
            ),
          ),
        Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (var index = 0; index < items.length; index++)
              Listener(
                key: itemKeys[index],
                behavior: HitTestBehavior.translucent,
                onPointerDown: handleLocalPointerDown,
                child: KeyedSubtree(key: items[index].key, child: items[index].child),
              ),
          ],
        ),
      ],
    );
  }
}

enum _TrackingSource { local, scope }
