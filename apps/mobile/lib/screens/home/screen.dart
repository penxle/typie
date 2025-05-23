import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_bold.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/home/__generated__/screen.req.gql.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class HomeScreen extends HookWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final pref = useService<Pref>();

    useEffect(() {
      final subscription = client
          .subscribe(GHomeScreen_SiteUpdateStream_SubscriptionReq((b) => b..vars.siteId = pref.siteId))
          .listen((_) {});

      return subscription.cancel;
    });

    return AutoTabsRouter(
      routes: const [EntityRouter(), SearchRoute(), InboxRoute(), ProfileRoute()],
      duration: Duration.zero,
      transitionBuilder: (context, child, animation) => child,
      builder: (context, child) {
        final padding = MediaQuery.viewPaddingOf(context);

        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Expanded(
              child: MediaQuery.removeViewPadding(context: context, removeBottom: true, child: child),
            ),
            Container(
              height: padding.bottom + 52,
              padding: Pad(horizontal: 24, bottom: padding.bottom),
              decoration: const BoxDecoration(
                border: Border(top: BorderSide(color: AppColors.gray_950)),
                color: AppColors.gray_50,
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const _Button(
                    index: 0,
                    icon: Icon(LucideBoldIcons.folder, size: 24, color: AppColors.gray_300),
                    activeIcon: Icon(LucideBoldIcons.folder, size: 24, color: AppColors.gray_700),
                  ),
                  const _Button(
                    index: 1,
                    icon: Icon(LucideBoldIcons.search, size: 24, color: AppColors.gray_300),
                    activeIcon: Icon(LucideBoldIcons.search, size: 24, color: AppColors.gray_700),
                  ),
                  Tappable(
                    padding: const Pad(horizontal: 16),
                    onTap: () async {
                      final result = await client.request(
                        GHomeScreen_CreatePost_MutationReq((b) => b..vars.input.siteId = pref.siteId),
                      );

                      if (context.mounted) {
                        await context.router.push(EditorRoute(slug: result.createPost.entity.slug));
                      }
                    },
                    child: const Icon(LucideBoldIcons.square_plus, size: 24, color: AppColors.gray_300),
                  ),
                  const _Button(
                    index: 2,
                    icon: Icon(LucideBoldIcons.bell, size: 24, color: AppColors.gray_300),
                    activeIcon: Icon(LucideBoldIcons.bell, size: 24, color: AppColors.gray_700),
                  ),
                  const _Button(
                    index: 3,
                    icon: Icon(LucideBoldIcons.circle_user_round, size: 24, color: AppColors.gray_300),
                    activeIcon: Icon(LucideBoldIcons.circle_user_round, size: 24, color: AppColors.gray_700),
                  ),
                ],
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
