import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';

class HapticReorderableListView {
  const HapticReorderableListView._();

  static Widget builder({
    required List<String> orderedIds,
    required IndexedWidgetBuilder itemBuilder,
    required ReorderCallback onReorder,
    ScrollController? scrollController,
    EdgeInsets? padding,
    bool buildDefaultDragHandles = true,
    ReorderItemProxyDecorator? proxyDecorator,
  }) {
    return _HapticReorderableListViewBuilder(
      orderedIds: orderedIds,
      itemBuilder: itemBuilder,
      onReorder: onReorder,
      scrollController: scrollController,
      padding: padding,
      buildDefaultDragHandles: buildDefaultDragHandles,
      proxyDecorator: proxyDecorator,
    );
  }
}

class _HapticReorderableListViewBuilder extends HookWidget {
  const _HapticReorderableListViewBuilder({
    required this.orderedIds,
    required this.itemBuilder,
    required this.onReorder,
    required this.scrollController,
    required this.padding,
    required this.buildDefaultDragHandles,
    required this.proxyDecorator,
  });

  final List<String> orderedIds;
  final IndexedWidgetBuilder itemBuilder;
  final ReorderCallback onReorder;
  final ScrollController? scrollController;
  final EdgeInsets? padding;
  final bool buildDefaultDragHandles;
  final ReorderItemProxyDecorator? proxyDecorator;

  @override
  Widget build(BuildContext context) {
    final tracker = useMemoized(_ReorderHapticTracker.new, [])..updateOrderedIds(orderedIds);

    useEffect(() => tracker.dispose, [tracker]);

    return Listener(
      onPointerDown: tracker.handlePointerDown,
      onPointerMove: tracker.handlePointerMove,
      onPointerUp: tracker.handlePointerUp,
      onPointerCancel: tracker.handlePointerCancel,
      child: ReorderableListView.builder(
        scrollController: scrollController,
        padding: padding,
        itemCount: orderedIds.length,
        buildDefaultDragHandles: buildDefaultDragHandles,
        proxyDecorator: proxyDecorator,
        onReorderStart: (index) {
          tracker.startDrag(index);
          unawaited(HapticFeedback.lightImpact());
        },
        onReorderEnd: (index) {
          tracker.endDrag();
          unawaited(HapticFeedback.lightImpact());
        },
        onReorder: onReorder,
        itemBuilder: (context, index) {
          final id = orderedIds[index];
          final child = itemBuilder(context, index);
          return KeyedSubtree(
            key: ValueKey<String>(id),
            child: _TrackedReorderItem(id: id, tracker: tracker, child: child),
          );
        },
      ),
    );
  }
}

class HapticReorderableList extends HookWidget {
  const HapticReorderableList({
    required this.orderedIds,
    required this.itemBuilder,
    required this.onReorder,
    this.controller,
    this.physics,
    this.padding,
    this.proxyDecorator,
    super.key,
  });

  final List<String> orderedIds;
  final IndexedWidgetBuilder itemBuilder;
  final ReorderCallback onReorder;
  final ScrollController? controller;
  final ScrollPhysics? physics;
  final EdgeInsets? padding;
  final ReorderItemProxyDecorator? proxyDecorator;

  @override
  Widget build(BuildContext context) {
    final tracker = useMemoized(_ReorderHapticTracker.new, [])..updateOrderedIds(orderedIds);

    useEffect(() => tracker.dispose, [tracker]);

    return Listener(
      onPointerDown: tracker.handlePointerDown,
      onPointerMove: tracker.handlePointerMove,
      onPointerUp: tracker.handlePointerUp,
      onPointerCancel: tracker.handlePointerCancel,
      child: ReorderableList(
        controller: controller,
        physics: physics,
        padding: padding,
        itemCount: orderedIds.length,
        proxyDecorator: proxyDecorator,
        onReorderStart: (index) {
          tracker.startDrag(index);
          unawaited(HapticFeedback.lightImpact());
        },
        onReorderEnd: (index) {
          tracker.endDrag();
          unawaited(HapticFeedback.lightImpact());
        },
        onReorder: onReorder,
        itemBuilder: (context, index) {
          final id = orderedIds[index];
          final child = itemBuilder(context, index);
          return KeyedSubtree(
            key: ValueKey<String>(id),
            child: _TrackedReorderItem(id: id, tracker: tracker, child: child),
          );
        },
      ),
    );
  }
}

class SliverHapticReorderableList extends HookWidget {
  const SliverHapticReorderableList({
    required this.orderedIds,
    required this.itemBuilder,
    required this.onReorder,
    this.proxyDecorator,
    super.key,
  });

  final List<String> orderedIds;
  final IndexedWidgetBuilder itemBuilder;
  final ReorderCallback onReorder;
  final ReorderItemProxyDecorator? proxyDecorator;

  @override
  Widget build(BuildContext context) {
    final tracker = useMemoized(_ReorderHapticTracker.new, [])..updateOrderedIds(orderedIds);

    useEffect(() => tracker.dispose, [tracker]);

    return SliverReorderableList(
      itemCount: orderedIds.length,
      onReorder: onReorder,
      onReorderStart: (index) {
        tracker.startDrag(index);
        unawaited(HapticFeedback.lightImpact());
      },
      onReorderEnd: (index) {
        tracker.endDrag();
        unawaited(HapticFeedback.lightImpact());
      },
      proxyDecorator: proxyDecorator,
      itemBuilder: (context, index) {
        final id = orderedIds[index];
        final child = itemBuilder(context, index);
        return KeyedSubtree(
          key: ValueKey<String>(id),
          child: Listener(
            onPointerDown: tracker.handlePointerDown,
            onPointerMove: tracker.handlePointerMove,
            onPointerUp: tracker.handlePointerUp,
            onPointerCancel: tracker.handlePointerCancel,
            child: _TrackedReorderItem(id: id, tracker: tracker, child: child),
          ),
        );
      },
    );
  }
}

class _TrackedReorderItem extends HookWidget {
  const _TrackedReorderItem({required this.id, required this.tracker, required this.child});

  final String id;
  final _ReorderHapticTracker tracker;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    useEffect(() {
      tracker.registerItemContext(id, context);
      return () => tracker.unregisterItemContext(id, context);
    });
    return child;
  }
}

class _ReorderHapticTracker {
  final _itemContexts = <String, BuildContext>{};

  List<String> _orderedIds = const [];
  int? _activePointer;
  Offset? _lastPointerGlobalPosition;
  String? _draggingId;
  int? _lastHoverSlot;
  bool _hasGlobalRoute = false;

  void registerItemContext(String id, BuildContext context) {
    _itemContexts[id] = context;
  }

  void unregisterItemContext(String id, BuildContext context) {
    if (_itemContexts[id] == context) {
      _itemContexts.remove(id);
    }
  }

  void updateOrderedIds(Iterable<String> ids) {
    _orderedIds = List<String>.unmodifiable(ids);
    final idSet = _orderedIds.toSet();
    _itemContexts.removeWhere((id, _) => !idSet.contains(id));
  }

  void handlePointerDown(PointerDownEvent event) {
    _activePointer = event.pointer;
    _lastPointerGlobalPosition = event.position;
  }

  void handlePointerMove(PointerMoveEvent event) {
    if (event.pointer != _activePointer) {
      return;
    }

    _lastPointerGlobalPosition = event.position;
  }

  void handlePointerUp(PointerUpEvent event) {
    if (event.pointer != _activePointer) {
      return;
    }

    _lastPointerGlobalPosition = event.position;
  }

  void handlePointerCancel(PointerCancelEvent event) {
    if (event.pointer != _activePointer) {
      return;
    }

    _activePointer = null;
    _lastPointerGlobalPosition = null;
  }

  void startDrag(int index) {
    if (index < 0 || index >= _orderedIds.length) {
      return;
    }

    _draggingId = _orderedIds[index];
    _lastHoverSlot = _computeHoverSlot(_lastPointerGlobalPosition);
    _attachGlobalRoute();
  }

  void endDrag() {
    _detachGlobalRoute();
    _draggingId = null;
    _lastHoverSlot = null;
  }

  void dispose() {
    endDrag();
  }

  void _attachGlobalRoute() {
    if (_hasGlobalRoute) {
      return;
    }

    GestureBinding.instance.pointerRouter.addGlobalRoute(_onGlobalPointerEvent);
    _hasGlobalRoute = true;
  }

  void _detachGlobalRoute() {
    if (!_hasGlobalRoute) {
      return;
    }

    GestureBinding.instance.pointerRouter.removeGlobalRoute(_onGlobalPointerEvent);
    _hasGlobalRoute = false;
  }

  void _onGlobalPointerEvent(PointerEvent event) {
    if (_draggingId == null || event.pointer != _activePointer) {
      return;
    }
    if (event is! PointerMoveEvent) {
      return;
    }

    _lastPointerGlobalPosition = event.position;

    final hoverSlot = _computeHoverSlot(event.position);
    if (hoverSlot == null || hoverSlot == _lastHoverSlot) {
      return;
    }

    _lastHoverSlot = hoverSlot;
    unawaited(HapticFeedback.lightImpact());
  }

  int? _computeHoverSlot(Offset? pointerPosition) {
    final draggingId = _draggingId;
    if (draggingId == null || pointerPosition == null) {
      return null;
    }

    final items = <_ReorderItemGeometry>[];
    for (final id in _orderedIds) {
      if (id == draggingId) {
        continue;
      }

      final context = _itemContexts[id];
      final renderBox = context?.findRenderObject() as RenderBox?;
      if (renderBox == null || !renderBox.attached || !renderBox.hasSize) {
        continue;
      }

      final topLeft = renderBox.localToGlobal(Offset.zero);
      final rect = topLeft & renderBox.size;
      items.add(_ReorderItemGeometry(rect: rect));
    }

    if (items.isEmpty) {
      return 0;
    }

    items.sort((a, b) => a.rect.top.compareTo(b.rect.top));

    var slot = 0;
    for (final item in items) {
      if (pointerPosition.dy > item.rect.center.dy) {
        slot++;
      }
    }

    return slot;
  }
}

class _ReorderItemGeometry {
  const _ReorderItemGeometry({required this.rect});

  final Rect rect;
}
