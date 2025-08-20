import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/services/auth.dart';

@RoutePage()
class GuardRouter extends HookWidget {
  const GuardRouter({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final state = useValueListenable(auth);

    if (state is AuthInitializing) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }

    return AutoRouter.declarative(
      routes: (handler) {
        return [
          switch (state) {
            Authenticated() => const DashboardRouter(),
            Unauthenticated() => const AuthRouter(),
            Offline() => const OfflineRoute(),
            _ => const AuthRouter(),
          },
        ];
      },
    );
  }
}
