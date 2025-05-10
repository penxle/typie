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
          page: authShell,
          transitionsBuilder: TransitionsBuilders.fadeIn,
          children: [AutoRoute(page: LoginWithEmailRoute.page, initial: true)],
        ),
        CustomRoute<dynamic>(
          page: dashboardShell,
          transitionsBuilder: TransitionsBuilders.fadeIn,
          children: [AutoRoute(page: HomeRoute.page, initial: true)],
        ),
      ],
    ),
  ];
}
