import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/native_editor/auto_discard.dart';
import 'package:typie/screens/shell/__generated__/create_document.req.gql.dart';
import 'package:typie/screens/shell/__generated__/site_update_stream.req.gql.dart';
import 'package:typie/services/site.dart';
import 'package:typie/widgets/responsive_container.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class ShellScreen extends HookWidget {
  const ShellScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final site = useService<Site>();
    final siteId = useValueListenable(site);
    final mixpanel = useService<Mixpanel>();

    useEffect(() {
      final subscription = client
          .subscribe(GHomeScreen_SiteUpdateStream_SubscriptionReq((b) => b..vars.siteId = siteId))
          .listen((_) {});

      return subscription.cancel;
    }, [siteId]);

    return AutoTabsRouter(
      routes: const [HomeRoute(), EntityRouter(), NotesRoute(), ProfileRoute()],
      duration: Duration.zero,
      transitionBuilder: (context, child, animation) => child,
      builder: (context, child) {
        final mediaQuery = MediaQuery.of(context);
        final screenMediaQuery = mediaQuery.copyWith(
          viewInsets: mediaQuery.viewInsets.copyWith(
            bottom: mediaQuery.viewInsets.bottom - mediaQuery.viewPadding.bottom - 52,
          ),
        );

        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Expanded(
              child: MediaQuery(data: screenMediaQuery, child: child),
            ),
            Container(
              height: mediaQuery.viewPadding.bottom + 52,
              padding: Pad(horizontal: 24, bottom: mediaQuery.viewPadding.bottom),
              decoration: BoxDecoration(
                border: Border(top: BorderSide(color: context.colors.borderDefault)),
                color: context.colors.surfaceDefault,
              ),
              child: ResponsiveContainer(
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    _Button(
                      index: 0,
                      icon: Icon(LucideLightIcons.house, size: 24, color: context.colors.textSubtle),
                      activeIcon: Icon(LucideLightIcons.house, size: 24, color: context.colors.textDefault),
                    ),
                    _Button(
                      index: 1,
                      icon: Icon(LucideLightIcons.folder_open, size: 24, color: context.colors.textSubtle),
                      activeIcon: Icon(TypieIcons.folder_open_filled, size: 24, color: context.colors.textDefault),
                    ),
                    Tappable(
                      padding: const Pad(horizontal: 16),
                      onTap: () async {
                        String? parentEntityId;

                        final topRoute = context.topRoute;
                        if (topRoute.name == EntityRoute.name) {
                          final args = topRoute.argsAs<EntityRouteArgs>(orElse: EntityRouteArgs.new);
                          parentEntityId = args.entityId;
                        }

                        final result = await client.request(
                          GHomeScreen_CreateDocument_MutationReq(
                            (b) => b
                              ..vars.input.siteId = site.siteId
                              ..vars.input.parentEntityId = Value.present(parentEntityId),
                          ),
                        );

                        unawaited(mixpanel.track('create_document', properties: {'via': 'home'}));

                        if (context.mounted) {
                          markAutoDiscardCandidate(result.createDocument.entity.slug);
                          await context.router.push(NativeEditorRoute(slug: result.createDocument.entity.slug));
                        }
                      },
                      child: Icon(LucideLightIcons.square_plus, size: 24, color: context.colors.textSubtle),
                    ),
                    _Button(
                      index: 2,
                      icon: Icon(LucideLightIcons.sticky_note, size: 24, color: context.colors.textSubtle),
                      activeIcon: Icon(TypieIcons.sticky_note_filled, size: 24, color: context.colors.textDefault),
                    ),
                    _Button(
                      index: 3,
                      icon: Icon(LucideLightIcons.circle_user_round, size: 24, color: context.colors.textSubtle),
                      activeIcon: Icon(
                        TypieIcons.circle_user_round_filled,
                        size: 24,
                        color: context.colors.textDefault,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}

class _Button extends StatelessWidget {
  const _Button({required this.index, required this.icon, this.activeIcon});

  final int index;
  final Widget icon;
  final Widget? activeIcon;

  @override
  Widget build(BuildContext context) {
    final tabsRouter = AutoTabsRouter.of(context, watch: true);

    if (tabsRouter.activeIndex == index) {
      return GestureDetector(
        behavior: HitTestBehavior.opaque,
        onDoubleTapDown: (_) {
          final router = context.router.topMostRouter();
          if (router is StackRouter) {
            router.popUntilRoot();
          }
        },
        child: Padding(padding: const Pad(horizontal: 16), child: activeIcon ?? icon),
      );
    } else {
      return GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTapDown: (_) {
          tabsRouter.setActiveIndex(index);
        },
        child: Padding(padding: const Pad(horizontal: 16), child: icon),
      );
    }
  }
}
