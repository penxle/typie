import 'dart:async';
import 'dart:ui' as ui;

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/native_editor/auto_discard.dart';
import 'package:typie/screens/shell/__generated__/create_document.req.gql.dart';
import 'package:typie/screens/shell/__generated__/site_update_stream.req.gql.dart';
import 'package:typie/screens/shell/nav.dart';
import 'package:typie/screens/shell/nav_fab.dart';
import 'package:typie/services/site.dart';
import 'package:typie/widgets/animated_toggle.dart';

@RoutePage()
class ShellScreen extends HookWidget {
  const ShellScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final site = useService<Site>();
    final siteId = useValueListenable(site);
    final mixpanel = useService<Mixpanel>();
    final crossFadeKey = useMemoized(GlobalKey<_SnapshotCrossFadeState>.new);

    useEffect(() {
      final subscription = client
          .subscribe(GHomeScreen_SiteUpdateStream_SubscriptionReq((b) => b..vars.siteId = siteId))
          .listen((_) {});

      return subscription.cancel;
    }, [siteId]);

    final navVisible = useMemoized(() => ValueNotifier(true));
    final navVisibleValue = useValueListenable(navVisible);
    final trailingAction = useMemoized(ShellTrailingActionController.new);
    final trailingActionValue = useValueListenable(trailingAction);
    final pillColor = context.theme.brightness == Brightness.dark
        ? context.colors.surfaceSubtle
        : context.colors.surfaceDefault;

    Future<void> createDocument() async {
      final result = await client.request(
        GHomeScreen_CreateDocument_MutationReq((b) => b..vars.input.siteId = site.siteId),
      );

      unawaited(mixpanel.track('create_document', properties: {'via': 'home'}));

      if (context.mounted) {
        markAutoDiscardCandidate(result.createDocument.entity.slug);
        await context.router.push(NativeEditorRoute(slug: result.createDocument.entity.slug));
      }
    }

    return ShellNav(
      visible: navVisible,
      trailingAction: trailingAction,
      child: AutoTabsRouter.builder(
        builder: (context, children, tabsRouter) {
          final effectiveTrailingAction = tabsRouter.activeIndex == 1 ? trailingActionValue : null;

          return Stack(
            children: [
              Positioned.fill(
                child: _SnapshotCrossFade(
                  key: crossFadeKey,
                  child: IndexedStack(index: tabsRouter.activeIndex, children: children),
                ),
              ),
              ShellNavFab(
                action: effectiveTrailingAction,
                navVisible: navVisibleValue,
                color: pillColor,
                onDefaultTap: createDocument,
              ),
              Positioned(
                left: 24,
                right: 24,
                bottom: MediaQuery.viewPaddingOf(context).bottom + 12,
                child: AnimatedSlide(
                  offset: Offset(0, navVisibleValue ? 0 : 2),
                  duration: const Duration(milliseconds: 300),
                  curve: Curves.easeOutCubic,
                  child: AnimatedOpacity(
                    opacity: navVisibleValue ? 1 : 0,
                    duration: const Duration(milliseconds: 200),
                    child: IgnorePointer(
                      ignoring: !navVisibleValue,
                      child: Center(
                        child: ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 488),
                          child: Row(
                            children: [
                              Expanded(
                                child: Container(
                                  height: 60,
                                  decoration: BoxDecoration(
                                    color: pillColor,
                                    borderRadius: BorderRadius.circular(30),
                                    border: Border.all(color: context.colors.borderDefault),
                                    boxShadow: [
                                      BoxShadow(color: context.colors.shadowAmbient, blurRadius: 8),
                                      BoxShadow(
                                        color: context.colors.shadowDefault,
                                        offset: const Offset(0, 4),
                                        blurRadius: 12,
                                      ),
                                      BoxShadow(
                                        color: context.colors.shadowDefault,
                                        offset: const Offset(0, 12),
                                        blurRadius: 32,
                                      ),
                                    ],
                                  ),
                                  child: Row(
                                    crossAxisAlignment: CrossAxisAlignment.stretch,
                                    children: [
                                      for (int i = 0; i < 4; i++)
                                        Expanded(
                                          child: _Button(
                                            index: i,
                                            activeIndex: tabsRouter.activeIndex,
                                            icon: Icon(
                                              [
                                                LucideIcons.house,
                                                LucideIcons.folder_open,
                                                LucideIcons.sticky_note,
                                                LucideIcons.circle_user_round,
                                              ][i],
                                              size: 24,
                                              color: context.colors.textSubtle,
                                            ),
                                            onTap: () {
                                              crossFadeKey.currentState?.captureAndAnimate();
                                              tabsRouter.setActiveIndex(i);
                                            },
                                          ),
                                        ),
                                    ],
                                  ),
                                ),
                              ),
                              const SizedBox(width: shellNavFabTotalWidth),
                            ],
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

class _SnapshotCrossFade extends StatefulWidget {
  const _SnapshotCrossFade({super.key, required this.child});

  final Widget child;

  @override
  State<_SnapshotCrossFade> createState() => _SnapshotCrossFadeState();
}

class _SnapshotCrossFadeState extends State<_SnapshotCrossFade> with SingleTickerProviderStateMixin {
  final _boundaryKey = GlobalKey();
  ui.Image? _snapshot;
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(vsync: this, duration: const Duration(milliseconds: 150))
      ..addStatusListener((status) {
        if (status == AnimationStatus.completed) {
          setState(() {
            _snapshot?.dispose();
            _snapshot = null;
          });
        }
      });
  }

  void captureAndAnimate() {
    final boundary = _boundaryKey.currentContext?.findRenderObject() as RenderRepaintBoundary?;
    if (boundary == null) {
      return;
    }

    try {
      _snapshot?.dispose();
      _snapshot = boundary.toImageSync(pixelRatio: MediaQuery.devicePixelRatioOf(context));
      unawaited(_controller.forward(from: 0));
      setState(() {});
    } catch (_) {
      // Boundary not ready, skip transition
    }
  }

  @override
  void dispose() {
    _snapshot?.dispose();
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      fit: StackFit.passthrough,
      children: [
        RepaintBoundary(key: _boundaryKey, child: widget.child),
        if (_snapshot != null)
          Positioned.fill(
            child: IgnorePointer(
              child: FadeTransition(
                opacity: Tween<double>(
                  begin: 1,
                  end: 0,
                ).animate(CurvedAnimation(parent: _controller, curve: Curves.easeOut)),
                child: RawImage(image: _snapshot, fit: BoxFit.cover),
              ),
            ),
          ),
      ],
    );
  }
}

class _Button extends StatelessWidget {
  const _Button({required this.index, required this.activeIndex, required this.icon, required this.onTap});

  final int index;
  final int activeIndex;
  final Widget icon;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final isActive = activeIndex == index;

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTapDown: isActive ? null : (_) => onTap(),
      child: Padding(
        padding: const Pad(all: 4),
        child: AnimatedToggle(
          value: isActive,
          builder: (context, t, child) {
            return Container(
              padding: const Pad(horizontal: 12),
              decoration: BoxDecoration(
                color: context.colors.surfaceMuted.withValues(alpha: context.colors.surfaceMuted.a * t),
                borderRadius: BorderRadius.circular(26),
              ),
              child: child,
            );
          },
          child: Center(child: icon),
        ),
      ),
    );
  }
}
