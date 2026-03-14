import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide.dart';
import 'package:typie/screens/shell/nav.dart';
import 'package:typie/widgets/popover/popover.dart';
import 'package:typie/widgets/tappable.dart';

const shellNavFabSize = 60.0;
const shellNavFabGap = 8.0;
const shellNavFabTotalWidth = shellNavFabSize + shellNavFabGap;

const _shellNavFabBottomOffset = 12.0;
const _shellNavFabMenuGap = 10.0;
const _shellNavFabMenuRadius = 22.0;
const _shellNavFabSelectionArmDelay = Duration(milliseconds: 150);
const _shellNavFabSamePressSelectionDistance = 9.0;

class ShellNavFab extends HookWidget {
  const ShellNavFab({
    required this.action,
    required this.navVisible,
    required this.color,
    required this.onDefaultTap,
    super.key,
  });

  final ShellTrailingActionConfig? action;
  final bool navVisible;
  final Color color;
  final Future<void> Function() onDefaultTap;

  @override
  Widget build(BuildContext context) {
    final isMenuOpen = useState(false);
    final menuController = useAnimationController(
      duration: const Duration(milliseconds: 280),
      reverseDuration: const Duration(milliseconds: 180),
    );
    final pointerNotifier = useMemoized(() => ValueNotifier<Object?>(null));
    final trackedPointer = useRef<int?>(null);
    final selectionArmTimer = useRef<Timer?>(null);
    final isTrackedPointerArmed = useRef(false);
    final isTrackedPointerHoldComplete = useRef(false);
    final trackedPointerOrigin = useRef<Offset?>(null);
    final lastTrackedPointerEvent = useRef<PointerEvent?>(null);
    final mediaQuery = MediaQuery.of(context);
    final shellWidth = math.min(mediaQuery.size.width - 48, 488);
    final shellHorizontalInset = (mediaQuery.size.width - shellWidth) / 2;
    final menuAnimation = useMemoized(
      () => CurvedAnimation(parent: menuController, curve: Curves.easeOutCubic, reverseCurve: Curves.easeInCubic),
      [menuController],
    );
    final hasPane = action?.pane != null;

    late final PointerRoute handleTrackedPointer;

    void updateTrackedPointerArmState(PointerEvent event) {
      if (isTrackedPointerArmed.value || !isTrackedPointerHoldComplete.value) {
        return;
      }

      final origin = trackedPointerOrigin.value;
      if (origin == null || (event.position - origin).distance <= _shellNavFabSamePressSelectionDistance) {
        return;
      }

      isTrackedPointerArmed.value = true;
    }

    void endTrackedPointer(int pointer) {
      if (trackedPointer.value != pointer) {
        return;
      }

      GestureBinding.instance.pointerRouter.removeGlobalRoute(handleTrackedPointer);
      trackedPointer.value = null;
      selectionArmTimer.value?.cancel();
      selectionArmTimer.value = null;
      isTrackedPointerArmed.value = false;
      isTrackedPointerHoldComplete.value = false;
      trackedPointerOrigin.value = null;
      lastTrackedPointerEvent.value = null;
      pointerNotifier.value = null;
    }

    void beginTrackedPointer(PointerDownEvent event) {
      if (trackedPointer.value != null) {
        return;
      }

      trackedPointer.value = event.pointer;
      isTrackedPointerArmed.value = false;
      isTrackedPointerHoldComplete.value = false;
      trackedPointerOrigin.value = event.position;
      lastTrackedPointerEvent.value = event;
      pointerNotifier.value = PopoverPointerState(event: event, isSelectionArmed: false);
      selectionArmTimer.value?.cancel();
      selectionArmTimer.value = Timer(_shellNavFabSelectionArmDelay, () {
        if (trackedPointer.value != event.pointer) {
          return;
        }

        isTrackedPointerHoldComplete.value = true;
        final latestEvent = lastTrackedPointerEvent.value;
        final previousArmed = isTrackedPointerArmed.value;
        if (latestEvent != null) {
          updateTrackedPointerArmState(latestEvent);
        }

        if (latestEvent != null && previousArmed != isTrackedPointerArmed.value) {
          pointerNotifier.value = PopoverPointerState(
            event: latestEvent,
            isSelectionArmed: isTrackedPointerArmed.value,
          );
        }
      });
      GestureBinding.instance.pointerRouter.addGlobalRoute(handleTrackedPointer);
    }

    useEffect(() {
      isMenuOpen.value = false;
      return null;
    }, [action]);

    useEffect(() {
      if (!navVisible) {
        isMenuOpen.value = false;
      }
      return null;
    }, [navVisible]);

    useEffect(() {
      if (isMenuOpen.value) {
        unawaited(menuController.forward());
      } else {
        unawaited(menuController.reverse());
      }
      return null;
    }, [isMenuOpen.value, menuController]);

    handleTrackedPointer = useMemoized<PointerRoute>(() {
      return (event) {
        if (event.pointer != trackedPointer.value) {
          return;
        }

        lastTrackedPointerEvent.value = event;
        updateTrackedPointerArmState(event);
        pointerNotifier.value = PopoverPointerState(event: event, isSelectionArmed: isTrackedPointerArmed.value);

        if (event is PointerUpEvent || event is PointerCancelEvent) {
          final pointer = event.pointer;
          scheduleMicrotask(() {
            endTrackedPointer(pointer);
          });
        }
      };
    });

    useEffect(() {
      return () {
        selectionArmTimer.value?.cancel();
        if (trackedPointer.value != null) {
          GestureBinding.instance.pointerRouter.removeGlobalRoute(handleTrackedPointer);
        }
        pointerNotifier.dispose();
      };
    }, [handleTrackedPointer, pointerNotifier]);

    return Positioned.fill(
      child: Stack(
        children: [
          if (isMenuOpen.value && hasPane)
            GestureDetector(
              behavior: HitTestBehavior.translucent,
              onTap: () {
                isMenuOpen.value = false;
              },
              child: const SizedBox.expand(),
            ),
          if (action case ShellTrailingActionConfig(pane: final pane?))
            Positioned(
              right: shellHorizontalInset,
              bottom: mediaQuery.viewPadding.bottom + _shellNavFabBottomOffset + shellNavFabSize + _shellNavFabMenuGap,
              child: _ShellNavFabMenu(
                visible: isMenuOpen.value,
                animation: menuAnimation,
                child: PopoverPointerScope(
                  notifier: pointerNotifier,
                  child: ShellTrailingActionMenuScope(
                    close: () {
                      isMenuOpen.value = false;
                    },
                    child: pane,
                  ),
                ),
              ),
            ),
          Positioned(
            right: shellHorizontalInset,
            bottom: mediaQuery.viewPadding.bottom + _shellNavFabBottomOffset,
            child: AnimatedSlide(
              offset: Offset(0, navVisible ? 0 : 2),
              duration: const Duration(milliseconds: 300),
              curve: Curves.easeOutCubic,
              child: AnimatedOpacity(
                opacity: navVisible ? 1 : 0,
                duration: const Duration(milliseconds: 200),
                child: IgnorePointer(
                  ignoring: !navVisible,
                  child: hasPane
                      ? Listener(
                          behavior: HitTestBehavior.opaque,
                          onPointerDown: (event) {
                            if (isMenuOpen.value) {
                              isMenuOpen.value = false;
                              return;
                            }

                            beginTrackedPointer(event);
                            isMenuOpen.value = true;
                          },
                          child: _ShellNavFabButton(
                            icon: isMenuOpen.value ? LucideIcons.x : action?.icon ?? LucideIcons.square_pen,
                            color: color,
                          ),
                        )
                      : Tappable(
                          onTap: () async {
                            await (action?.onTap?.call() ?? onDefaultTap());
                          },
                          child: _ShellNavFabButton(icon: action?.icon ?? LucideIcons.square_pen, color: color),
                        ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _ShellNavFabButton extends StatelessWidget {
  const _ShellNavFabButton({required this.icon, required this.color});

  final IconData icon;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: shellNavFabSize,
      height: shellNavFabSize,
      decoration: BoxDecoration(
        color: color,
        shape: BoxShape.circle,
        border: Border.all(color: context.colors.borderDefault),
        boxShadow: [
          BoxShadow(color: context.colors.shadowAmbient, blurRadius: 8),
          BoxShadow(color: context.colors.shadowDefault, offset: const Offset(0, 4), blurRadius: 12),
          BoxShadow(color: context.colors.shadowDefault, offset: const Offset(0, 12), blurRadius: 32),
        ],
      ),
      child: Icon(icon, size: 24, color: context.colors.textSubtle),
    );
  }
}

class _ShellNavFabMenu extends StatelessWidget {
  const _ShellNavFabMenu({required this.visible, required this.animation, required this.child});

  final bool visible;
  final Animation<double> animation;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final shape = RoundedSuperellipseBorder(
      borderRadius: BorderRadius.circular(_shellNavFabMenuRadius),
      side: BorderSide(color: context.colors.borderDefault),
    );
    final offset = animation.drive(Tween(begin: const Offset(0, 0.12), end: Offset.zero));

    return IgnorePointer(
      ignoring: !visible,
      child: FadeTransition(
        opacity: animation,
        child: SlideTransition(
          position: offset,
          child: Material(
            color: context.colors.surfaceDefault,
            elevation: 8,
            shadowColor: context.colors.shadowDefault.withValues(alpha: 0.08),
            shape: shape,
            clipBehavior: Clip.antiAlias,
            child: child,
          ),
        ),
      ),
    );
  }
}
