import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/modals/plan.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/profile/__generated__/screen.req.gql.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class ProfileScreen extends HookWidget {
  const ProfileScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();

    return Screen(
      heading: const Heading(title: '프로필', titleIcon: LucideLightIcons.circle_user_round),
      child: GraphQLOperation(
        operation: GProfileScreen_QueryReq(),
        builder: (context, client, data) {
          return Container(
            padding: const Pad(all: 24),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              spacing: 16,
              children: [
                Container(
                  decoration: BoxDecoration(
                    border: Border.all(color: AppColors.gray_950),
                    borderRadius: BorderRadius.circular(8),
                    color: AppColors.white,
                  ),
                  padding: const Pad(horizontal: 16, vertical: 24),
                  child: Column(
                    spacing: 2,
                    children: [
                      Tappable(
                        onTap: () async {
                          await context.router.push(const UpdateProfileRoute());
                        },
                        child: ClipOval(
                          child: CachedNetworkImage(
                            imageUrl: '${data.me!.avatar.url}?s=80&q=75',
                            width: 80,
                            height: 80,
                            fit: BoxFit.cover,
                            fadeInDuration: const Duration(milliseconds: 150),
                          ),
                        ),
                      ),
                      const Gap(8),
                      Tappable(
                        onTap: () async {
                          await context.router.push(const UpdateProfileRoute());
                        },
                        child: Row(
                          mainAxisAlignment: MainAxisAlignment.center,
                          spacing: 4,
                          children: [
                            Flexible(
                              child: Text(
                                data.me!.name,
                                style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                            const Icon(LucideLightIcons.pencil, size: 14, color: AppColors.gray_500),
                          ],
                        ),
                      ),
                      Text(data.me!.email, style: const TextStyle(fontSize: 14, color: AppColors.gray_500)),
                    ],
                  ),
                ),
                Tappable(
                  child: const Text('plan'),
                  onTap: () async {
                    await context.showBottomSheet(child: const PlanModal());
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
