import 'package:auto_route/auto_route.dart';
import 'package:typie/routers/app.gr.dart';

@AutoRouterConfig()
class AppRouter extends RootStackRouter {
  @override
  final defaultRouteType = const RouteType.cupertino();

  @override
  late final List<AutoRoute> routes = [
    AutoRoute(
      page: GuardRouter.page,
      initial: true,
      children: [
        CustomRoute<dynamic>(
          page: AuthRouter.page,
          transitionsBuilder: TransitionsBuilders.fadeIn,
          children: [
            AutoRoute(page: LoginRoute.page, initial: true),
            AutoRoute(page: LoginWithEmailRoute.page),
          ],
        ),
        CustomRoute<dynamic>(
          page: DashboardRouter.page,
          transitionsBuilder: TransitionsBuilders.fadeIn,
          children: [
            AutoRoute(
              page: HomeRoute.page,
              initial: true,
              children: [
                AutoRoute(
                  page: EntityRouter.page,
                  children: [AutoRoute(page: EntityRoute.page, initial: true)],
                ),
                AutoRoute(page: SearchRoute.page),
                AutoRoute(page: InboxRoute.page),
                AutoRoute(page: ProfileRoute.page),
              ],
            ),
            AutoRoute(page: EditorRoute.page, fullscreenDialog: true),
          ],
        ),
      ],
    ),
  ];
}
