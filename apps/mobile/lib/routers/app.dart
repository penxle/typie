import 'package:auto_route/auto_route.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/routers/parallax_route.dart';

@AutoRouterConfig()
class AppRouter extends RootStackRouter {
  @override
  final defaultRouteType = RouteType.custom(
    customRouteBuilder: <T>(context, child, page) {
      final fullScreenBackGesture = page.routeData.name == EntityRoute.name;
      return ParallaxPageRoute<T>(
        content: child,
        fullScreenBackGesture: fullScreenBackGesture,
        settings: page,
        fullscreenDialog: page.fullscreenDialog,
      );
    },
  );

  @override
  late final routes = <AutoRoute>[
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
        CustomRoute<dynamic>(page: OfflineRoute.page, transitionsBuilder: TransitionsBuilders.fadeIn),
        CustomRoute<dynamic>(page: MaintenanceRoute.page, transitionsBuilder: TransitionsBuilders.fadeIn),
        CustomRoute<dynamic>(page: UpdateRequiredRoute.page, transitionsBuilder: TransitionsBuilders.fadeIn),
        CustomRoute<dynamic>(
          page: DashboardRouter.page,
          transitionsBuilder: TransitionsBuilders.fadeIn,
          children: [
            AutoRoute(
              page: ShellRoute.page,
              initial: true,
              children: [
                AutoRoute(page: HomeRoute.page, initial: true),
                AutoRoute(
                  page: EntityRouter.page,
                  children: [AutoRoute(page: EntityRoute.page, initial: true)],
                ),
                AutoRoute(page: NotesRoute.page),
                AutoRoute(page: ProfileRoute.page),
              ],
            ),
            AutoRoute(page: SettingsRoute.page),
            AutoRoute(page: SocialAccountsRoute.page),
            AutoRoute(page: EditorSettingsRoute.page),
            AutoRoute(page: UpdateProfileRoute.page),
            AutoRoute(page: UpdatePasswordRoute.page),
            AutoRoute(page: UpdateEmailRoute.page),
            AutoRoute(page: SiteRoute.page),
            AutoRoute(page: SiteSettingsRoute.page),
            AutoRoute(page: CurrentPlanRoute.page),
            AutoRoute(page: EnrollPlanRoute.page),
            AutoRoute(page: CancelPlanRoute.page),
            AutoRoute(page: DeleteUserRoute.page),
            AutoRoute(page: OssLicensesRoute.page),
            AutoRoute(page: ReferralRoute.page),
            AutoRoute(page: StatsRoute.page),
            AutoRoute(page: NativeEditorRoute.page, fullscreenDialog: true),
            AutoRoute(page: TrashRoute.page),
            AutoRoute(page: TextReplacementsRoute.page),
          ],
        ),
      ],
    ),
  ];
}
