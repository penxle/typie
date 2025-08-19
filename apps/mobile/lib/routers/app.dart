import 'package:auto_route/auto_route.dart';
import 'package:typie/routers/app.gr.dart';

@AutoRouterConfig()
class AppRouter extends RootStackRouter {
  @override
  final defaultRouteType = const RouteType.cupertino();

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
            AutoRoute(page: SettingsRoute.page),
            AutoRoute(page: SocialAccountsRoute.page),
            AutoRoute(page: EditorSettingsRoute.page),
            AutoRoute(page: UpdateProfileRoute.page),
            AutoRoute(page: UpdateEmailRoute.page),
            AutoRoute(page: UpdateSiteSlugRoute.page),
            AutoRoute(page: CurrentPlanRoute.page),
            AutoRoute(page: EnrollPlanRoute.page),
            AutoRoute(page: CancelPlanRoute.page),
            AutoRoute(page: DeleteUserRoute.page),
            AutoRoute(page: OssLicensesRoute.page),
            AutoRoute(page: ReferralRoute.page),
            AutoRoute(page: EditorRoute.page, fullscreenDialog: true),
            AutoRoute(page: CanvasRoute.page, fullscreenDialog: true),
            AutoRoute(page: TrashRoute.page),
          ],
        ),
      ],
    ),
  ];
}
