<script lang="ts">
  import * as PortOne from '@portone/browser-sdk/v2';
  import { graphql } from '$graphql';

  const finalizeIdentityVerification = graphql(`
    mutation IdPage_FinalizeIdentityVerification_Mutation($input: FinalizeIdentityVerificationInput!) {
      finalizeIdentityVerification(input: $input) {
        id
      }
    }
  `);

  const handle = async () => {
    const resp = await PortOne.requestIdentityVerification({
      storeId: 'store-e1e69136-38bb-42dd-b226-3c78e03c1ff1',
      identityVerificationId: `identity-verification-${crypto.randomUUID()}`,
      channelKey: 'channel-key-31e03361-26cb-4810-86ed-801cce4f570f',
    });

    if (resp === undefined) {
      console.log('error');
      return;
    }

    await finalizeIdentityVerification({ identityVerificationId: resp.identityVerificationId });
  };
</script>

<button onclick={handle} type="button">
  <span>테스트</span>
</button>
