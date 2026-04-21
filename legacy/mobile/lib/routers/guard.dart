import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/services/bootstrap.dart';

@RoutePage()
class GuardRouter extends HookWidget {
  const GuardRouter({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final authState = useValueListenable(auth);

    final bootstrap = useService<BootstrapService>();
    final bootstrapState = useValueListenable(bootstrap);

    if (authState is AuthInitializing || bootstrapState is BootstrapLoading) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }

    return AutoRouter.declarative(
      routes: (handler) {
        return [
          switch (bootstrapState) {
            BootstrapMaintenance(:final title, :final message, :final until) => MaintenanceRoute(
              title: title,
              message: message,
              until: until,
            ),
            BootstrapUpdateRequired(:final storeUrl, :final currentVersion, :final requiredVersion) =>
              UpdateRequiredRoute(storeUrl: storeUrl, currentVersion: currentVersion, requiredVersion: requiredVersion),
            _ => switch (authState) {
              Authenticated() => const DashboardRouter(),
              Unauthenticated() => const AuthRouter(),
              Offline() => const OfflineRoute(),
              _ => const AuthRouter(),
            },
          },
        ];
      },
    );
  }
}
