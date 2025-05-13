import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:in_app_purchase/in_app_purchase.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/providers/__generated__/in_app_purchase_enroll_plan_with_in_app_purchase_mutation.req.gql.dart';

class InAppPurchaseProvider extends HookWidget {
  const InAppPurchaseProvider({super.key});

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();

    useEffect(() {
      final subscription = InAppPurchase.instance.purchaseStream.listen((purchaseDetailsList) async {
        for (final purchaseDetails in purchaseDetailsList) {
          try {
            if (purchaseDetails.status == PurchaseStatus.purchased) {
              await client.request(
                GInAppPurchase_EnrollPlanWithInAppPurchase_MutationReq((b) {
                  b.vars.input.store = Platform.isIOS ? GInAppPurchaseStore.APP_STORE : GInAppPurchaseStore.GOOGLE_PLAY;
                  b.vars.input.data =
                      Platform.isIOS
                          ? purchaseDetails.purchaseID
                          : purchaseDetails.verificationData.serverVerificationData;
                }),
              );
            }
          } on Exception {
            // pass
          } finally {
            if (purchaseDetails.pendingCompletePurchase) {
              await InAppPurchase.instance.completePurchase(purchaseDetails);
            }
          }
        }
      });

      return subscription.cancel;
    });

    return const SizedBox.shrink();
  }
}
