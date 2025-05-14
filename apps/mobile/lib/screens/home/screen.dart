import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide.dart';
import 'package:typie/modals/plan.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/home/__generated__/query.req.gql.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/widgets/btn.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class HomeScreen extends HookWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();

    return Screen(
      child: GraphQLOperation(
        operation: GHomeScreen_QueryReq(),
        builder: (context, client, data) {
          return Center(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              spacing: 16,
              children: [
                const Icon(LucideIcons.user, size: 100),
                Text(data.me!.email),
                Btn(
                  'editor',
                  onTap: () async {
                    await context.router.push(const EditorRoute());
                  },
                ),
                Tappable(
                  child: const Text('plan'),
                  onTap: () async {
                    await context.showBottomSheet(const PlanModal());
                  },
                ),
                Tappable(
                  child: const Text('Entity Tree'),
                  onTap: () async {
                    await context.router.push(EntityTreeRoute(entityId: null));
                  },
                ),
                Tappable(
                  child: const Text('logout'),
                  onTap: () async {
                    await auth.logout();
                  },
                ),
              ],
            ),
          );
        },
      ),
    );
  }
}
