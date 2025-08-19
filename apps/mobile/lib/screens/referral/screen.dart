import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:share_plus/share_plus.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/referral/__generated__/referral_mutation.req.gql.dart';
import 'package:typie/screens/referral/__generated__/referral_query.req.gql.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class ReferralScreen extends StatelessWidget {
  const ReferralScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Screen(
      heading: const Heading(title: '초대'),
      child: GraphQLOperation(
        operation: GReferralScreen_QueryReq(),
        builder: (context, client, data) {
          final referralCount = data.me?.referrals.length ?? 0;
          final compensatedCount = data.me?.referrals.where((r) => r.compensated).length ?? 0;

          return SingleChildScrollView(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              spacing: 24,
              children: [
                _Section(
                  title: '친구 초대',
                  children: [
                    Padding(
                      padding: const EdgeInsets.all(16),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        spacing: 16,
                        children: [
                          Text(
                            '친구를 초대하면 친구는 바로 1달 무료, 친구가 첫 결제를 하면 나도 1달 무료 혜택을 받아요.',
                            style: TextStyle(fontSize: 14, color: context.colors.textDefault),
                          ),
                          Row(
                            spacing: 8,
                            children: [
                              Tappable(
                                onTap: () async {
                                  final issueReferralUrl = GReferralScreen_IssueReferralUrl_MutationReq();
                                  final response = await client.request(issueReferralUrl);

                                  final referralUrl = response.issueReferralUrl;
                                  final message = '📝 타이피 가입하고 한달 무료 혜택 받아가세요! $referralUrl';

                                  await Clipboard.setData(ClipboardData(text: message));

                                  if (context.mounted) {
                                    context.toast(ToastType.success, '초대 링크가 복사되었어요.');
                                  }
                                },
                                child: Container(
                                  decoration: BoxDecoration(
                                    borderRadius: BorderRadius.circular(8),
                                    border: Border.all(color: context.colors.borderStrong),
                                  ),
                                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                                  child: const Row(
                                    mainAxisSize: MainAxisSize.min,
                                    spacing: 8,
                                    children: [
                                      Icon(LucideLightIcons.copy, size: 16),
                                      Text('초대 링크 복사', style: TextStyle(fontSize: 14)),
                                    ],
                                  ),
                                ),
                              ),
                              Tappable(
                                onTap: () async {
                                  final issueReferralUrl = GReferralScreen_IssueReferralUrl_MutationReq();
                                  final response = await client.request(issueReferralUrl);

                                  final referralUrl = response.issueReferralUrl;
                                  final message = '📝 타이피 가입하고 한달 무료 혜택 받아가세요! $referralUrl';

                                  await SharePlus.instance.share(ShareParams(title: message, text: message));
                                },
                                child: Container(
                                  decoration: BoxDecoration(
                                    borderRadius: BorderRadius.circular(8),
                                    border: Border.all(color: context.colors.borderStrong),
                                  ),
                                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                                  child: const Row(
                                    mainAxisSize: MainAxisSize.min,
                                    spacing: 8,
                                    children: [
                                      Icon(LucideLightIcons.share, size: 16),
                                      Text('공유하기', style: TextStyle(fontSize: 14)),
                                    ],
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ],
                ),

                _Section(
                  title: '초대 현황',
                  children: [
                    Padding(
                      padding: const EdgeInsets.all(16),
                      child: Column(
                        spacing: 12,
                        children: [
                          Container(
                            decoration: BoxDecoration(
                              borderRadius: BorderRadius.circular(8),
                              color: context.colors.surfaceSubtle,
                              border: Border.all(color: context.colors.borderDefault),
                            ),
                            padding: const EdgeInsets.all(16),
                            child: Row(
                              children: [
                                Icon(LucideLightIcons.users, size: 18, color: context.colors.textSubtle),
                                const SizedBox(width: 8),
                                Expanded(
                                  child: Text(
                                    '초대한 친구',
                                    style: TextStyle(fontSize: 15, color: context.colors.textSubtle),
                                  ),
                                ),
                                Text(
                                  '$referralCount명',
                                  style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                                ),
                              ],
                            ),
                          ),
                          Container(
                            decoration: BoxDecoration(
                              borderRadius: BorderRadius.circular(8),
                              color: context.colors.surfaceSubtle,
                              border: Border.all(color: context.colors.borderDefault),
                            ),
                            padding: const EdgeInsets.all(16),
                            child: Row(
                              children: [
                                Icon(LucideLightIcons.gift, size: 18, color: context.colors.textSubtle),
                                const SizedBox(width: 8),
                                Expanded(
                                  child: Text(
                                    '받은 혜택',
                                    style: TextStyle(fontSize: 15, color: context.colors.textSubtle),
                                  ),
                                ),
                                Text(
                                  compensatedCount > 0 ? '$compensatedCount개월 무료' : '없음',
                                  style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                                ),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),

                Container(
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(8),
                    color: context.colors.surfaceSubtle,
                  ),
                  padding: const EdgeInsets.only(left: 4, right: 4, bottom: 32),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    spacing: 12,
                    children: [
                      const Text('초대 혜택 안내', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
                      Column(
                        spacing: 8,
                        children: [
                          _buildBulletPoint(
                            context,
                            '초대 링크를 통해 웹에서 가입하고, 웹에서 플랜을 가입해야 초대 혜택을 받을 수 있어요. 앱에서 가입하면 혜택을 받을 수 없어요.',
                          ),
                          _buildBulletPoint(
                            context,
                            '친구가 초대 링크로 가입하면 친구는 즉시 FULL ACCESS 플랜 1개월에 해당하는 크레딧을 지급받아요. 지급받은 크레딧으로 바로 FULL ACCESS 플랜을 체험해볼 수 있어요.',
                          ),
                          _buildBulletPoint(
                            context,
                            '친구가 크레딧을 통한 체험을 끝내고 첫 결제를 완료하면 나도 FULL ACCESS 플랜 1개월에 상응하는 크레딧을 지급받아요. 이 크레딧은 다음 FULL ACCESS 플랜 갱신시 자동으로 이용돼요.',
                          ),
                          _buildBulletPoint(context, '초대 횟수에는 제한이 없어요.'),
                        ],
                      ),
                    ],
                  ),
                ),
              ],
            ),
          );
        },
      ),
    );
  }

  Widget _buildBulletPoint(BuildContext context, String text) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 8,
      children: [
        Container(
          margin: const EdgeInsets.only(top: 9),
          width: 4,
          height: 4,
          decoration: BoxDecoration(color: context.colors.textFaint, shape: BoxShape.circle),
        ),
        Expanded(
          child: Text(text, style: TextStyle(fontSize: 14, color: context.colors.textFaint, height: 1.6)),
        ),
      ],
    );
  }
}

class _Section extends StatelessWidget {
  const _Section({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 8,
      children: [
        Text(
          title,
          style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textFaint),
        ),
        Container(
          decoration: BoxDecoration(
            border: Border.all(color: context.colors.borderStrong),
            borderRadius: BorderRadius.circular(8),
            color: context.colors.surfaceDefault,
          ),
          child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: children),
        ),
      ],
    );
  }
}
