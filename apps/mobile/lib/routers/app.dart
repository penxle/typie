import 'package:auto_route/auto_route.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/routers/shells.dart';

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
          page: AuthShell,
          transitionsBuilder: TransitionsBuilders.fadeIn,
          children: [AutoRoute(page: LoginRoute.page, initial: true), AutoRoute(page: LoginWithEmailRoute.page)],
        ),
        CustomRoute<dynamic>(
          page: DashboardShell,
          transitionsBuilder: TransitionsBuilders.fadeIn,
          children: [
            AutoRoute(page: HomeRoute.page, initial: true),
            AutoRoute(page: EditorRoute.page, fullscreenDialog: true),
            AutoRoute(page: EntityTreeRoute.page, path: 'entity/:entityId'),
          ],
        ),
      ],
    ),
  ];
}
