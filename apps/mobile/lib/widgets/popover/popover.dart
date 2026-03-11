import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';

enum PopoverPosition { bottomLeft, bottomCenter, bottomRight, topLeft, topCenter, topRight }

class PopoverPaneTransition {
  const PopoverPaneTransition({required this.progress, required this.anchorContentRect});

  final double progress;
  final Rect anchorContentRect;
}

class PopoverPointerState {
  const PopoverPointerState({required this.event, required this.isSelectionArmed});

  final PointerEvent event;
  final bool isSelectionArmed;
}

class PopoverPointerScope extends InheritedNotifier<ValueNotifier<Object?>> {
  const PopoverPointerScope({required super.notifier, required super.child, super.key});

  static ValueListenable<Object?>? maybeOf(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<PopoverPointerScope>();
    return scope?.notifier;
  }
}

class PopoverPaneTransitionScope extends InheritedWidget {
  const PopoverPaneTransitionScope({required this.transition, required super.child, super.key});

  final PopoverPaneTransition transition;

  static PopoverPaneTransition? maybeOf(BuildContext context) {
    return context.dependOnInheritedWidgetOfExactType<PopoverPaneTransitionScope>()?.transition;
  }

  @override
  bool updateShouldNotify(covariant PopoverPaneTransitionScope oldWidget) {
    return transition.progress != oldWidget.transition.progress ||
        transition.anchorContentRect != oldWidget.transition.anchorContentRect;
  }
}

class Popover extends StatefulWidget {
  const Popover({
    required this.anchor,
    required this.pane,
    this.position = PopoverPosition.bottomRight,
    this.maxWidth,
    this.screenPadding = const EdgeInsets.all(16),
    this.collapsedBorderRadius = BorderRadius.zero,
    this.expandedBorderRadius = defaultExpandedBorderRadius,
    this.backgroundColor,
    this.borderSide,
    super.key,
  });
  static const expandedRadius = 22.0;
  static const panePadding = 6.0;
  static const defaultExpandedBorderRadius = BorderRadius.all(Radius.circular(expandedRadius));

  final Widget anchor;
  final Widget pane;
  final PopoverPosition position;
  final double? maxWidth;
  final EdgeInsets screenPadding;
  final BorderRadius collapsedBorderRadius;
  final BorderRadius expandedBorderRadius;
  final Color? backgroundColor;
  final BorderSide? borderSide;

  static void close(BuildContext context) {
    context.getInheritedWidgetOfExactType<_PopoverCloseScope>()?.close();
  }

  @override
  State<Popover> createState() => _PopoverState();
}

class _PopoverState extends State<Popover> with SingleTickerProviderStateMixin {
  static const _selectionArmDelay = Duration(milliseconds: 150);
  static const _samePressSelectionDistance = 9.0;
  static const _popoverCurve = Curves.easeOutExpo;
  static const _fadeCurve = Curves.easeOutExpo;

  final _anchorKey = GlobalKey();
  final _paneKey = GlobalKey();
  final _surfaceKey = GlobalKey();
  final _overlayController = OverlayPortalController();
  final _anchorPointerNotifier = ValueNotifier<Object?>(null);

  late final AnimationController _animationController = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 320),
    reverseDuration: const Duration(milliseconds: 240),
  )..addStatusListener(_handleAnimationStatusChanged);

  bool _isExpanded = false;
  bool _isOverlayVisible = false;
  bool _isOutsidePointerRouteRegistered = false;
  bool _isAnchorMeasurementScheduled = false;
  bool _isMeasurementScheduled = false;
  final Map<int, _PopoverOutsideTapRecognizer> _outsideTapRecognizers = {};
  ScrollHoldController? _anchorScrollHold;
  int? _trackedAnchorPointer;
  Timer? _anchorSelectionArmTimer;
  bool _isTrackedAnchorPointerArmed = false;
  bool _isTrackedAnchorPointerHoldComplete = false;
  Offset? _trackedAnchorPointerOrigin;
  PointerEvent? _lastTrackedAnchorPointerEvent;
  Rect? _lastAnchorRect;
  Size? _paneSize;

  Widget _buildPane(BuildContext context) {
    return _PopoverCloseScope(close: _close, child: widget.pane);
  }

  Decoration? _buildDecoration(BuildContext context, BorderRadius borderRadius, {double shadowOpacity = 1}) {
    if (widget.backgroundColor == null && widget.borderSide == null) {
      return null;
    }

    return ShapeDecoration(
      color: widget.backgroundColor,
      shadows: shadowOpacity <= 0
          ? null
          : [
              BoxShadow(
                color: context.colors.shadowDefault.withValues(alpha: 0.08 * shadowOpacity),
                offset: const Offset(0, 4),
                blurRadius: 12,
              ),
            ],
      shape: RoundedSuperellipseBorder(borderRadius: borderRadius, side: widget.borderSide ?? BorderSide.none),
    );
  }

  void _handleAnimationStatusChanged(AnimationStatus status) {
    if (status == AnimationStatus.dismissed && !_isExpanded) {
      _removeOutsidePointerRoute();
      _overlayController.hide();
      _lastAnchorRect = null;
      if (mounted) {
        setState(() {
          _isOverlayVisible = false;
        });
      }
    }
  }

  void _handlePointerDown(PointerDownEvent event) {
    _beginAnchorPointerTracking(event);
    _open();
  }

  void _beginAnchorPointerTracking(PointerDownEvent event) {
    if (_trackedAnchorPointer != null) {
      return;
    }

    final anchorContext = _anchorKey.currentContext;
    final scrollable = anchorContext == null ? null : Scrollable.maybeOf(anchorContext);
    _anchorScrollHold ??= scrollable?.position.hold(_handleAnchorScrollHoldCanceled);
    _trackedAnchorPointer = event.pointer;
    _isTrackedAnchorPointerArmed = false;
    _isTrackedAnchorPointerHoldComplete = false;
    _trackedAnchorPointerOrigin = event.position;
    _lastTrackedAnchorPointerEvent = event;
    _anchorPointerNotifier.value = PopoverPointerState(event: event, isSelectionArmed: false);
    _anchorSelectionArmTimer?.cancel();
    _anchorSelectionArmTimer = Timer(_selectionArmDelay, () {
      if (_trackedAnchorPointer != event.pointer) {
        return;
      }

      _isTrackedAnchorPointerHoldComplete = true;
      final latestEvent = _lastTrackedAnchorPointerEvent;
      final previousArmed = _isTrackedAnchorPointerArmed;
      if (latestEvent != null) {
        _updateTrackedAnchorPointerArmState(latestEvent);
      }
      if (latestEvent != null && previousArmed != _isTrackedAnchorPointerArmed) {
        _anchorPointerNotifier.value = PopoverPointerState(event: latestEvent, isSelectionArmed: true);
      }
    });
    GestureBinding.instance.pointerRouter.addGlobalRoute(_handleAnchorPointerEvent);
  }

  void _updateTrackedAnchorPointerArmState(PointerEvent event) {
    if (_isTrackedAnchorPointerArmed || !_isTrackedAnchorPointerHoldComplete) {
      return;
    }

    final origin = _trackedAnchorPointerOrigin;
    if (origin == null || (event.position - origin).distance <= _samePressSelectionDistance) {
      return;
    }

    _isTrackedAnchorPointerArmed = true;
  }

  void _handleAnchorScrollHoldCanceled() {
    _anchorScrollHold = null;
  }

  void _releaseAnchorScrollHold() {
    final scrollHold = _anchorScrollHold;
    _anchorScrollHold = null;
    scrollHold?.cancel();
  }

  void _endAnchorPointerTracking(int pointer) {
    if (_trackedAnchorPointer != pointer) {
      return;
    }

    GestureBinding.instance.pointerRouter.removeGlobalRoute(_handleAnchorPointerEvent);
    _trackedAnchorPointer = null;
    _anchorSelectionArmTimer?.cancel();
    _anchorSelectionArmTimer = null;
    _isTrackedAnchorPointerArmed = false;
    _isTrackedAnchorPointerHoldComplete = false;
    _trackedAnchorPointerOrigin = null;
    _lastTrackedAnchorPointerEvent = null;
    _anchorPointerNotifier.value = null;
    _releaseAnchorScrollHold();
  }

  void _handleAnchorPointerEvent(PointerEvent event) {
    if (event.pointer != _trackedAnchorPointer) {
      return;
    }

    _lastTrackedAnchorPointerEvent = event;
    _updateTrackedAnchorPointerArmState(event);
    _anchorPointerNotifier.value = PopoverPointerState(event: event, isSelectionArmed: _isTrackedAnchorPointerArmed);

    if (event is PointerUpEvent || event is PointerCancelEvent) {
      final pointer = event.pointer;
      scheduleMicrotask(() {
        _endAnchorPointerTracking(pointer);
      });
    }
  }

  void _open() {
    if (_isOverlayVisible) {
      return;
    }

    _addOutsidePointerRoute();
    setState(() {
      _isExpanded = true;
      _isOverlayVisible = true;
    });
    _animationController.value = 0;
    _overlayController.show();

    if (_paneSize != null) {
      unawaited(_animationController.forward(from: 0));
    }
  }

  void _close() {
    if (!_isOverlayVisible) {
      return;
    }

    setState(() {
      _isExpanded = false;
    });

    if (_paneSize == null) {
      _removeOutsidePointerRoute();
      _overlayController.hide();
      _lastAnchorRect = null;
      setState(() {
        _isOverlayVisible = false;
      });
      return;
    }

    final reverseFrom = _animationController.value == 0 ? 1.0 : _animationController.value;
    unawaited(_animationController.reverse(from: reverseFrom));
  }

  bool _handleAnchorLayoutChanged(SizeChangedLayoutNotification notification) {
    if (_isOverlayVisible) {
      _scheduleAnchorMeasurement();
    }
    return false;
  }

  Rect? _currentAnchorRect() {
    final renderObject = _anchorKey.currentContext?.findRenderObject();
    if (renderObject is! RenderBox || !renderObject.hasSize) {
      return null;
    }

    return renderObject.localToGlobal(Offset.zero) & renderObject.size;
  }

  void _scheduleAnchorMeasurement() {
    if (_isAnchorMeasurementScheduled) {
      return;
    }

    _isAnchorMeasurementScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _isAnchorMeasurementScheduled = false;
      if (!mounted || !_isOverlayVisible) {
        return;
      }

      final anchorRect = _currentAnchorRect();
      if (anchorRect == null) {
        return;
      }

      final previous = _lastAnchorRect;
      _lastAnchorRect = anchorRect;
      if (previous != null && !_rectEquals(previous, anchorRect)) {
        setState(() {});
      }
    });
  }

  void _schedulePaneMeasurement() {
    if (_isMeasurementScheduled) {
      return;
    }

    _isMeasurementScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _isMeasurementScheduled = false;
      if (!mounted || !_isOverlayVisible) {
        return;
      }

      final renderObject = _paneKey.currentContext?.findRenderObject();
      if (renderObject is! RenderBox || !renderObject.hasSize) {
        return;
      }

      final size = renderObject.size;
      final previous = _paneSize;
      final hasChanged =
          previous == null || (previous.width - size.width).abs() > 0.5 || (previous.height - size.height).abs() > 0.5;

      if (hasChanged && _isExpanded) {
        setState(() {
          _paneSize = size;
        });
      }

      if (_isExpanded && !_animationController.isAnimating && _animationController.value == 0) {
        unawaited(_animationController.forward(from: 0));
      }
    });
  }

  void _addOutsidePointerRoute() {
    if (_isOutsidePointerRouteRegistered) {
      return;
    }

    GestureBinding.instance.pointerRouter.addGlobalRoute(_handleOutsidePointerEvent);
    _isOutsidePointerRouteRegistered = true;
  }

  void _removeOutsidePointerRoute() {
    if (!_isOutsidePointerRouteRegistered) {
      return;
    }

    GestureBinding.instance.pointerRouter.removeGlobalRoute(_handleOutsidePointerEvent);
    _isOutsidePointerRouteRegistered = false;
  }

  Rect? _currentPaneSurfaceRect() {
    final renderObject = _surfaceKey.currentContext?.findRenderObject();
    if (renderObject is! RenderBox || !renderObject.hasSize) {
      return null;
    }

    return renderObject.localToGlobal(Offset.zero) & renderObject.size;
  }

  bool _isEventInsidePaneSurface(PointerEvent event) {
    final paneRect = _currentPaneSurfaceRect();
    return paneRect != null && paneRect.contains(event.position);
  }

  void _trackOutsideTap(PointerDownEvent event) {
    if (_outsideTapRecognizers.containsKey(event.pointer)) {
      return;
    }

    final recognizer = _PopoverOutsideTapRecognizer(
      onTrackingEnded: () {
        _outsideTapRecognizers.remove(event.pointer)?.dispose();
      },
    );
    _outsideTapRecognizers[event.pointer] = recognizer;
    recognizer.addPointer(event);
  }

  void _handleOutsidePointerEvent(PointerEvent event) {
    if (!_isExpanded || _isEventInsidePaneSurface(event)) {
      return;
    }

    if (event is PointerDownEvent) {
      if (event.pointer == _trackedAnchorPointer) {
        return;
      }

      _trackOutsideTap(event);
      _close();
      return;
    }

    if (event is PointerPanZoomStartEvent || event is PointerSignalEvent) {
      _close();
    }
  }

  @override
  void dispose() {
    if (_trackedAnchorPointer != null) {
      GestureBinding.instance.pointerRouter.removeGlobalRoute(_handleAnchorPointerEvent);
    }
    _removeOutsidePointerRoute();
    for (final recognizer in _outsideTapRecognizers.values) {
      recognizer.dispose();
    }
    _outsideTapRecognizers.clear();
    _anchorSelectionArmTimer?.cancel();
    _releaseAnchorScrollHold();
    _anchorPointerNotifier.dispose();
    _animationController
      ..removeStatusListener(_handleAnimationStatusChanged)
      ..dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final shouldHideAnchorSemantics = _isOverlayVisible;

    return PopScope(
      canPop: !_isOverlayVisible,
      onPopInvokedWithResult: (didPop, result) {
        if (!didPop) {
          _close();
        }
      },
      child: OverlayPortal(
        controller: _overlayController,
        overlayChildBuilder: _buildOverlayChild,
        child: KeyedSubtree(
          key: _anchorKey,
          child: Semantics(
            button: !shouldHideAnchorSemantics,
            hidden: shouldHideAnchorSemantics,
            onTap: shouldHideAnchorSemantics ? null : _open,
            child: IgnorePointer(
              ignoring: _isOverlayVisible,
              child: RawGestureDetector(
                behavior: HitTestBehavior.translucent,
                gestures: {
                  _ImmediatePointerCaptureGestureRecognizer:
                      GestureRecognizerFactoryWithHandlers<_ImmediatePointerCaptureGestureRecognizer>(
                        _ImmediatePointerCaptureGestureRecognizer.new,
                        (recognizer) {},
                      ),
                },
                child: Listener(
                  behavior: HitTestBehavior.translucent,
                  onPointerDown: _handlePointerDown,
                  child: AnimatedBuilder(
                    animation: _animationController,
                    child: SizeChangedLayoutNotifier(child: widget.anchor),
                    builder: (context, child) {
                      return NotificationListener<SizeChangedLayoutNotification>(
                        onNotification: _handleAnchorLayoutChanged,
                        child: Opacity(opacity: _anchorOpacity, child: child),
                      );
                    },
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildOverlayChild(BuildContext context) {
    final anchorContext = _anchorKey.currentContext;
    final anchorRenderObject = anchorContext?.findRenderObject();
    if (anchorRenderObject is! RenderBox || !anchorRenderObject.hasSize) {
      return const SizedBox.shrink();
    }

    final scrollPosition = anchorContext == null ? null : Scrollable.maybeOf(anchorContext)?.position;
    if (scrollPosition == null) {
      return _buildOverlayPane(context, anchorRenderObject);
    }

    return ListenableBuilder(
      listenable: scrollPosition,
      builder: (context, child) {
        final updatedRenderObject = anchorContext?.findRenderObject();
        if (updatedRenderObject is! RenderBox || !updatedRenderObject.hasSize) {
          return const SizedBox.shrink();
        }

        return _buildOverlayPane(context, updatedRenderObject);
      },
    );
  }

  Widget _buildOverlayPane(BuildContext context, RenderBox anchorRenderObject) {
    _schedulePaneMeasurement();

    final anchorRect = anchorRenderObject.localToGlobal(Offset.zero) & anchorRenderObject.size;
    _lastAnchorRect = anchorRect;
    final mediaQuery = MediaQuery.of(context);
    final screenPadding = EdgeInsets.fromLTRB(
      widget.screenPadding.left + mediaQuery.padding.left,
      widget.screenPadding.top + mediaQuery.padding.top,
      widget.screenPadding.right + mediaQuery.padding.right,
      widget.screenPadding.bottom + mediaQuery.padding.bottom,
    );
    final paneSize = _paneSize;
    final showBelow = paneSize == null
        ? _prefersBottom(widget.position)
        : _shouldShowBelow(
            position: widget.position,
            childHeight: paneSize.height,
            overlayHeight: mediaQuery.size.height,
            anchorRect: anchorRect,
            screenPadding: screenPadding,
          );
    final effectivePosition = _effectivePosition(widget.position, showBelow);

    return Positioned.fill(
      child: Stack(
        children: [
          CustomSingleChildLayout(
            delegate: _PopoverLayoutDelegate(
              anchorRect: anchorRect,
              position: widget.position,
              maxWidth: widget.maxWidth,
              screenPadding: screenPadding,
            ),
            child: _buildAnimatedPane(context, anchorRect, effectivePosition),
          ),
        ],
      ),
    );
  }

  double get _anchorOpacity {
    if (!_isOverlayVisible || _paneSize == null) {
      return 1;
    }

    return 1 - _fadeCurve.transform(_animationController.value);
  }

  Widget _buildAnimatedPane(BuildContext context, Rect anchorRect, PopoverPosition effectivePosition) {
    final paneContent = _buildPane(context);
    final paneSize = _paneSize;

    if (paneSize == null) {
      return PopoverPointerScope(
        notifier: _anchorPointerNotifier,
        child: Opacity(
          opacity: 0,
          child: _FloatingPaneSurface(
            key: _surfaceKey,
            borderRadius: widget.collapsedBorderRadius,
            decoration: _buildDecoration(context, widget.collapsedBorderRadius, shadowOpacity: 0),
            child: KeyedSubtree(key: _paneKey, child: paneContent),
          ),
        ),
      );
    }

    return PopoverPointerScope(
      notifier: _anchorPointerNotifier,
      child: AnimatedBuilder(
        animation: _animationController,
        builder: (context, child) {
          final curvedProgress = _popoverCurve.transform(_animationController.value);
          final clampedProgress = curvedProgress.clamp(0.0, 1.0);
          final animatedWidth = _sizeForProgress(anchorRect.width, paneSize.width, curvedProgress);
          final animatedHeight = _sizeForProgress(anchorRect.height, paneSize.height, curvedProgress);
          final borderRadius = BorderRadius.lerp(
            widget.collapsedBorderRadius,
            widget.expandedBorderRadius,
            clampedProgress,
          )!;
          final contentOpacity = _fadeCurve.transform(_animationController.value);
          final anchorContentOpacity = 1 - contentOpacity;
          final anchorContentRect = _anchorContentRect(paneSize, anchorRect.size, effectivePosition);
          final paneTransition = PopoverPaneTransition(progress: clampedProgress, anchorContentRect: anchorContentRect);
          final paneChild = child!;
          final shouldCropContent = !_isExpanded || _animationController.isAnimating || _animationController.value < 1;
          final measuredChild = shouldCropContent
              ? SizedBox(
                  key: _paneKey,
                  width: paneSize.width,
                  height: paneSize.height,
                  child: PopoverPaneTransitionScope(transition: paneTransition, child: paneChild),
                )
              : KeyedSubtree(
                  key: _paneKey,
                  child: PopoverPaneTransitionScope(transition: paneTransition, child: paneChild),
                );
          final animatedChild = shouldCropContent
              ? OverflowBox(
                  alignment: _paneAlignment(effectivePosition),
                  minWidth: paneSize.width,
                  maxWidth: paneSize.width,
                  minHeight: paneSize.height,
                  maxHeight: paneSize.height,
                  child: Opacity(opacity: contentOpacity, child: measuredChild),
                )
              : Opacity(opacity: contentOpacity, child: measuredChild);

          return SizedBox(
            width: paneSize.width,
            height: paneSize.height,
            child: Stack(
              clipBehavior: Clip.none,
              children: [
                OverflowBox(
                  minWidth: 0,
                  maxWidth: double.infinity,
                  minHeight: 0,
                  maxHeight: double.infinity,
                  alignment: _paneAlignment(effectivePosition),
                  child: SizedBox(
                    width: animatedWidth,
                    height: animatedHeight,
                    child: _FloatingPaneSurface(
                      key: _surfaceKey,
                      borderRadius: borderRadius,
                      decoration: _buildDecoration(context, borderRadius, shadowOpacity: contentOpacity),
                      child: animatedChild,
                    ),
                  ),
                ),
                Positioned.fromRect(
                  rect: anchorContentRect,
                  child: IgnorePointer(
                    child: Opacity(opacity: anchorContentOpacity, child: widget.anchor),
                  ),
                ),
              ],
            ),
          );
        },
        child: paneContent,
      ),
    );
  }

  double _lerp(double begin, double end, double t) {
    return begin + ((end - begin) * t);
  }

  double _sizeForProgress(double begin, double end, double progress) {
    final size = _lerp(begin, end, progress);
    if (begin <= end) {
      return math.max(begin, size);
    }

    return math.min(begin, size);
  }
}

class _PopoverOutsideTapRecognizer extends PrimaryPointerGestureRecognizer {
  _PopoverOutsideTapRecognizer({required this.onTrackingEnded}) : super(allowedButtonsFilter: _allowAnyButton);

  final VoidCallback onTrackingEnded;
  bool _didFinishTracking = false;

  static bool _allowAnyButton(int buttons) => true;

  @override
  void handlePrimaryPointer(PointerEvent event) {
    if (event is PointerUpEvent) {
      resolve(GestureDisposition.accepted);
      return;
    }

    if (event is PointerCancelEvent) {
      resolve(GestureDisposition.rejected);
    }
  }

  @override
  void didStopTrackingLastPointer(int pointer) {
    super.didStopTrackingLastPointer(pointer);
    _finishTracking();
  }

  void _finishTracking() {
    if (_didFinishTracking) {
      return;
    }

    _didFinishTracking = true;
    onTrackingEnded();
  }

  @override
  String get debugDescription => 'popover outside tap';
}

class _PopoverCloseScope extends InheritedWidget {
  const _PopoverCloseScope({required this.close, required super.child});

  final VoidCallback close;

  @override
  bool updateShouldNotify(covariant _PopoverCloseScope oldWidget) {
    return close != oldWidget.close;
  }
}

Rect _anchorContentRect(Size paneSize, Size anchorSize, PopoverPosition position) {
  final left = switch (position) {
    PopoverPosition.bottomLeft || PopoverPosition.topLeft => 0.0,
    PopoverPosition.bottomCenter || PopoverPosition.topCenter => (paneSize.width - anchorSize.width) / 2,
    PopoverPosition.bottomRight || PopoverPosition.topRight => paneSize.width - anchorSize.width,
  };
  final top = switch (position) {
    PopoverPosition.bottomLeft || PopoverPosition.bottomCenter || PopoverPosition.bottomRight => 0.0,
    PopoverPosition.topLeft ||
    PopoverPosition.topCenter ||
    PopoverPosition.topRight => paneSize.height - anchorSize.height,
  };

  return Rect.fromLTWH(left, top, anchorSize.width, anchorSize.height);
}

bool _rectEquals(Rect a, Rect b) {
  return (a.left - b.left).abs() <= 0.5 &&
      (a.top - b.top).abs() <= 0.5 &&
      (a.width - b.width).abs() <= 0.5 &&
      (a.height - b.height).abs() <= 0.5;
}

Alignment _paneAlignment(PopoverPosition position) {
  return switch (position) {
    PopoverPosition.bottomLeft => Alignment.topLeft,
    PopoverPosition.bottomCenter => Alignment.topCenter,
    PopoverPosition.bottomRight => Alignment.topRight,
    PopoverPosition.topLeft => Alignment.bottomLeft,
    PopoverPosition.topCenter => Alignment.bottomCenter,
    PopoverPosition.topRight => Alignment.bottomRight,
  };
}

class _FloatingPaneSurface extends StatelessWidget {
  const _FloatingPaneSurface({required this.borderRadius, required this.child, this.decoration, super.key});

  final BorderRadius borderRadius;
  final Decoration? decoration;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final clippedChild = ClipRSuperellipse(borderRadius: borderRadius, child: child);

    if (decoration == null) {
      return clippedChild;
    }

    return DecoratedBox(decoration: decoration!, child: clippedChild);
  }
}

class _PopoverLayoutDelegate extends SingleChildLayoutDelegate {
  const _PopoverLayoutDelegate({
    required this.anchorRect,
    required this.position,
    required this.maxWidth,
    required this.screenPadding,
  });

  final Rect anchorRect;
  final PopoverPosition position;
  final double? maxWidth;
  final EdgeInsets screenPadding;

  @override
  BoxConstraints getConstraintsForChild(BoxConstraints constraints) {
    final safeWidth = math.max<double>(0, constraints.maxWidth - screenPadding.horizontal);
    final safeHeight = math.max<double>(0, constraints.maxHeight - screenPadding.vertical);
    final resolvedMaxWidth = maxWidth == null ? safeWidth : math.min(maxWidth!, safeWidth);

    return BoxConstraints(maxWidth: resolvedMaxWidth, maxHeight: safeHeight);
  }

  @override
  Offset getPositionForChild(Size size, Size childSize) {
    final showBelow = _shouldShowBelow(
      position: position,
      childHeight: childSize.height,
      overlayHeight: size.height,
      anchorRect: anchorRect,
      screenPadding: screenPadding,
    );
    final unclampedLeft = switch (position) {
      PopoverPosition.bottomLeft || PopoverPosition.topLeft => anchorRect.left,
      PopoverPosition.bottomCenter || PopoverPosition.topCenter => anchorRect.center.dx - (childSize.width / 2),
      PopoverPosition.bottomRight || PopoverPosition.topRight => anchorRect.right - childSize.width,
    };
    final unclampedTop = showBelow ? anchorRect.top : anchorRect.bottom - childSize.height;
    final minLeft = switch (position) {
      PopoverPosition.bottomLeft || PopoverPosition.topLeft => 0.0,
      _ => screenPadding.left,
    };
    final maxLeft = switch (position) {
      PopoverPosition.bottomRight || PopoverPosition.topRight => size.width - childSize.width,
      _ => size.width - screenPadding.right - childSize.width,
    };
    final minTop = showBelow ? 0.0 : screenPadding.top;
    final maxTop = showBelow ? size.height - screenPadding.bottom - childSize.height : size.height - childSize.height;

    return Offset(_clamp(unclampedLeft, minLeft, maxLeft), _clamp(unclampedTop, minTop, maxTop));
  }

  double _clamp(double value, double min, double max) {
    if (max < min) {
      return min;
    }

    return value.clamp(min, max);
  }

  @override
  bool shouldRelayout(_PopoverLayoutDelegate oldDelegate) {
    return anchorRect != oldDelegate.anchorRect ||
        position != oldDelegate.position ||
        maxWidth != oldDelegate.maxWidth ||
        screenPadding != oldDelegate.screenPadding;
  }
}

bool _prefersBottom(PopoverPosition position) {
  return switch (position) {
    PopoverPosition.bottomLeft || PopoverPosition.bottomCenter || PopoverPosition.bottomRight => true,
    _ => false,
  };
}

bool _shouldShowBelow({
  required PopoverPosition position,
  required double childHeight,
  required double overlayHeight,
  required Rect anchorRect,
  required EdgeInsets screenPadding,
}) {
  final bottomSpace = overlayHeight - screenPadding.bottom - anchorRect.top;
  final topSpace = anchorRect.bottom - screenPadding.top;
  final prefersBottom = _prefersBottom(position);

  if (prefersBottom) {
    if (childHeight <= bottomSpace) {
      return true;
    }
    if (childHeight <= topSpace) {
      return false;
    }
    return bottomSpace >= topSpace;
  }

  if (childHeight <= topSpace) {
    return false;
  }
  if (childHeight <= bottomSpace) {
    return true;
  }
  return bottomSpace > topSpace;
}

PopoverPosition _effectivePosition(PopoverPosition position, bool showBelow) {
  return switch ((position, showBelow)) {
    (PopoverPosition.bottomLeft, false) => PopoverPosition.topLeft,
    (PopoverPosition.bottomCenter, false) => PopoverPosition.topCenter,
    (PopoverPosition.bottomRight, false) => PopoverPosition.topRight,
    (PopoverPosition.topLeft, true) => PopoverPosition.bottomLeft,
    (PopoverPosition.topCenter, true) => PopoverPosition.bottomCenter,
    (PopoverPosition.topRight, true) => PopoverPosition.bottomRight,
    _ => position,
  };
}

class _ImmediatePointerCaptureGestureRecognizer extends OneSequenceGestureRecognizer {
  @override
  void addAllowedPointer(PointerDownEvent event) {
    startTrackingPointer(event.pointer);
    resolve(GestureDisposition.accepted);
  }

  @override
  void handleEvent(PointerEvent event) {
    if (event is PointerUpEvent || event is PointerCancelEvent) {
      stopTrackingPointer(event.pointer);
    }
  }

  @override
  void acceptGesture(int pointer) {}

  @override
  void rejectGesture(int pointer) {}

  @override
  String get debugDescription => 'immediatePointerCapture';

  @override
  void didStopTrackingLastPointer(int pointer) {}
}
