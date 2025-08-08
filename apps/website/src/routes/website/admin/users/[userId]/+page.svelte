<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import ArrowLeftIcon from '~icons/lucide/arrow-left';
  import { graphql } from '$graphql';
  import { AdminIcon, AdminModal } from '$lib/components/admin';

  let impersonateModalOpen = $state(false);

  const query = graphql(`
    query AdminUserDetail_Query($userId: String!) {
      adminUser(userId: $userId) {
        id
        name
        email
        role
        state
        createdAt
        avatar {
          id
          url
        }
        sites {
          id
          name
          url
        }
        singleSignOns {
          id
          provider
          email
        }
        subscription {
          id
          state
          startsAt
          expiresAt
          plan {
            id
            name
            availability
          }
        }
        credit
        personalIdentity {
          id
          name
          birthDate
          gender
          phoneNumber
        }
        marketingConsent
        recentPosts {
          id
          title
          createdAt
          updatedAt
        }
        postCount
        totalCharacterCount
        billingKey {
          id
          name
        }
      }
    }
  `);

  const adminImpersonate = graphql(`
    mutation AdminUserDetail_AdminImpersonate_Mutation($input: AdminImpersonateInput!) {
      adminImpersonate(input: $input)
    }
  `);

  const handleImpersonate = async () => {
    await adminImpersonate({ userId: $query.adminUser.id });
    location.href = '/home';
  };

  const adminGiveCredit = graphql(`
    mutation AdminUserDetail_AdminGiveCredit_Mutation($input: AdminGiveCreditInput!) {
      adminGiveCredit(input: $input)
    }
  `);
</script>

<div class={flex({ flexDirection: 'column', gap: '24px', color: 'amber.500' })}>
  <div class={flex({ alignItems: 'center', gap: '12px' })}>
    <button
      class={css({
        borderWidth: '2px',
        borderColor: 'amber.500',
        paddingX: '12px',
        paddingY: '6px',
        fontSize: '12px',
        color: 'amber.500',
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        backgroundColor: 'transparent',
        _hover: {
          backgroundColor: 'amber.500',
          color: 'gray.900',
        },
      })}
      onclick={() => history.back()}
      type="button"
    >
      <AdminIcon icon={ArrowLeftIcon} size={16} />
      BACK TO LIST
    </button>
    <h2 class={css({ fontSize: '18px', color: 'amber.500' })}>USER DETAILS</h2>
  </div>

  {#if $query.adminUser}
    <div
      class={grid({
        gap: '24px',
        gridTemplateColumns: '2fr 1fr',
        alignItems: 'start',
      })}
    >
      <!-- 왼쪽 컬럼: 핵심 콘텐츠 -->
      <div class={flex({ flexDirection: 'column', gap: '24px' })}>
        <!-- PROFILE -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>PROFILE</h3>

          <div class={flex({ gap: '20px', marginBottom: '24px' })}>
            <div
              class={css({
                size: '80px',
                backgroundColor: 'amber.500',
                overflow: 'hidden',
                flexShrink: '0',
              })}
            >
              {#if $query.adminUser.avatar?.url}
                <img alt={$query.adminUser.name} src={$query.adminUser.avatar.url} />
              {/if}
            </div>

            <div class={flex({ flexDirection: 'column', gap: '8px' })}>
              <h4 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'amber.500' })}>
                {$query.adminUser.name}
              </h4>
              <div class={css({ fontSize: '12px', color: 'amber.400' })}>
                {$query.adminUser.email}
              </div>
            </div>
          </div>
        </div>

        <!-- ACTIVITY -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>ACTIVITY</h3>

          <div class={grid({ gridTemplateColumns: 'repeat(2, 1fr)', gap: '24px', marginBottom: '32px' })}>
            <div>
              <div class={css({ fontSize: '24px', color: 'amber.500', marginBottom: '4px' })}>
                {$query.adminUser.postCount}
              </div>
              <div class={css({ fontSize: '11px', color: 'amber.400' })}>POSTS</div>
            </div>

            <div>
              <div class={css({ fontSize: '24px', color: 'amber.500', marginBottom: '4px' })}>
                {comma($query.adminUser.totalCharacterCount)}
              </div>
              <div class={css({ fontSize: '11px', color: 'amber.400' })}>CHARACTERS</div>
            </div>
          </div>

          <!-- RECENT POSTS -->
          <div>
            <h4 class={css({ fontSize: '14px', color: 'amber.500', marginBottom: '16px' })}>RECENT POSTS</h4>
            {#if $query.adminUser.recentPosts.length > 0}
              <div class={flex({ flexDirection: 'column', gap: '12px' })}>
                {#each $query.adminUser.recentPosts as post (post.id)}
                  <div
                    class={css({
                      borderWidth: '1px',
                      borderColor: 'amber.500',
                      padding: '12px',
                    })}
                  >
                    <a
                      class={css({
                        fontSize: '12px',
                        color: 'amber.500',
                        _hover: { textDecoration: 'underline' },
                      })}
                      href="/admin/posts/{post.id}"
                    >
                      {post.title || '(UNTITLED)'}
                    </a>
                    <div class={css({ fontSize: '10px', color: 'amber.400', marginTop: '4px' })}>
                      {dayjs(post.createdAt).formatAsDateTime()}
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              <div class={css({ fontSize: '12px', color: 'gray.400' })}>NO POSTS CREATED</div>
            {/if}
          </div>
        </div>

        <!-- SITES -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>
            SITES ({$query.adminUser.sites.length})
          </h3>

          {#if $query.adminUser.sites.length > 0}
            <div class={flex({ flexDirection: 'column', gap: '12px' })}>
              {#each $query.adminUser.sites as site (site.id)}
                <div
                  class={css({
                    borderWidth: '1px',
                    borderColor: 'amber.500',
                    padding: '16px',
                  })}
                >
                  <a
                    class={css({
                      fontSize: '14px',
                      fontWeight: 'bold',
                      color: 'amber.500',
                      _hover: { textDecoration: 'underline' },
                      display: 'block',
                      marginBottom: '4px',
                    })}
                    href={site.url}
                    rel="noopener noreferrer"
                    target="_blank"
                  >
                    {site.name}
                  </a>
                  <div class={css({ fontSize: '12px', color: 'amber.400' })}>
                    {site.url}
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <div class={css({ fontSize: '12px', color: 'gray.400' })}>NO SITES OWNED</div>
          {/if}
        </div>
      </div>

      <!-- 오른쪽 컬럼: 메타데이터 -->
      <div class={flex({ flexDirection: 'column', gap: '24px' })}>
        <!-- METADATA -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>METADATA</h3>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>USER ID</span>
              <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                {$query.adminUser.id}
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>ROLE</span>
              <span class={css({ fontSize: '12px', color: $query.adminUser.role === 'ADMIN' ? 'amber.500' : 'gray.400' })}>
                [{$query.adminUser.role}]
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>STATE</span>
              <span
                class={css({
                  fontSize: '12px',
                  color: $query.adminUser.state === 'ACTIVE' ? 'green.400' : 'red.400',
                })}
              >
                [{$query.adminUser.state}]
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>JOINED</span>
              <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                {dayjs($query.adminUser.createdAt).formatAsDateTime()}
              </span>
            </div>
          </div>
        </div>

        <!-- AUTHENTICATION -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>AUTHENTICATION</h3>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div>
              <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '8px' })}>LOGIN METHODS</div>
              {#if $query.adminUser.singleSignOns.length > 0}
                <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                  {#each $query.adminUser.singleSignOns as sso (sso.id)}
                    <div class={css({ fontSize: '12px', color: 'amber.500' })}>
                      [{sso.provider}] {sso.email}
                    </div>
                  {/each}
                </div>
              {:else}
                <div class={css({ fontSize: '12px', color: 'amber.500' })}>
                  [EMAIL] {$query.adminUser.email}
                </div>
              {/if}
            </div>
          </div>
        </div>

        <!-- IDENTITY -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>IDENTITY</h3>

          {#if $query.adminUser.personalIdentity}
            <div class={flex({ flexDirection: 'column', gap: '16px' })}>
              <div>
                <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>NAME</div>
                <div class={css({ fontSize: '14px', color: 'amber.500', fontWeight: 'bold' })}>
                  {$query.adminUser.personalIdentity.name}
                </div>
              </div>

              <div class={grid({ gridTemplateColumns: '1fr 1fr', gap: '16px' })}>
                <div>
                  <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>BIRTH DATE</div>
                  <div class={css({ fontSize: '12px', color: 'amber.500' })}>
                    {dayjs($query.adminUser.personalIdentity.birthDate).format('YYYY-MM-DD')}
                  </div>
                </div>
                <div>
                  <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>GENDER</div>
                  <div class={css({ fontSize: '12px', color: 'amber.500' })}>
                    [{$query.adminUser.personalIdentity.gender}]
                  </div>
                </div>
              </div>

              {#if $query.adminUser.personalIdentity.phoneNumber}
                <div>
                  <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>PHONE NUMBER</div>
                  <div class={css({ fontSize: '12px', color: 'amber.500' })}>
                    {$query.adminUser.personalIdentity.phoneNumber}
                  </div>
                </div>
              {/if}
            </div>
          {:else}
            <div class={css({ fontSize: '12px', color: 'gray.400', textAlign: 'center', paddingY: '24px' })}>NO IDENTITY VERIFICATION</div>
          {/if}
        </div>

        <!-- SUBSCRIPTION -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>SUBSCRIPTION</h3>

          {#if $query.adminUser.subscription}
            <div class={flex({ flexDirection: 'column', gap: '16px' })}>
              <div>
                <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>PLAN</div>
                <div class={css({ fontSize: '14px', color: 'amber.500', fontWeight: 'bold' })}>
                  {$query.adminUser.subscription.plan.name}
                </div>
              </div>

              <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
                <span class={css({ fontSize: '11px', color: 'amber.400' })}>STATUS</span>
                <span
                  class={css({
                    fontSize: '12px',
                    color:
                      $query.adminUser.subscription.state === 'ACTIVE'
                        ? 'green.400'
                        : $query.adminUser.subscription.state === 'WILL_EXPIRE'
                          ? 'amber.400'
                          : $query.adminUser.subscription.state === 'IN_GRACE_PERIOD'
                            ? 'red.400'
                            : 'gray.400',
                  })}
                >
                  [{$query.adminUser.subscription.state}]
                </span>
              </div>

              <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
                <span class={css({ fontSize: '11px', color: 'amber.400' })}>STARTED</span>
                <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                  {dayjs($query.adminUser.subscription.startsAt).formatAsDateTime()}
                </span>
              </div>

              <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
                <span class={css({ fontSize: '11px', color: 'amber.400' })}>EXPIRES</span>
                <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                  {dayjs($query.adminUser.subscription.expiresAt).formatAsDateTime()}
                </span>
              </div>

              <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
                <span class={css({ fontSize: '11px', color: 'amber.400' })}>PAYMENT METHOD</span>
                <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                  [{$query.adminUser.subscription.plan.availability}]
                </span>
              </div>
            </div>
          {:else}
            <div class={css({ fontSize: '12px', color: 'gray.400', textAlign: 'center', paddingY: '24px' })}>NO ACTIVE SUBSCRIPTION</div>
          {/if}
        </div>

        <!-- PAYMENT -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>PAYMENT</h3>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>BILLING KEY</span>
              {#if $query.adminUser.billingKey}
                <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                  {$query.adminUser.billingKey.name}
                </span>
              {:else}
                <span class={css({ fontSize: '12px', color: 'gray.400' })}>NONE</span>
              {/if}
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>CREDIT BALANCE</span>
              <span class={css({ fontSize: '12px', color: $query.adminUser.credit === 0 ? 'gray.400' : 'amber.500' })}>
                ₩{comma($query.adminUser.credit)}
              </span>
            </div>
          </div>
        </div>

        <!-- PREFERENCES -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>PREFERENCES</h3>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>MARKETING</span>
              <span
                class={css({
                  fontSize: '12px',
                  color: $query.adminUser.marketingConsent ? 'green.400' : 'gray.400',
                })}
              >
                {$query.adminUser.marketingConsent ? 'CONSENTED' : 'NOT CONSENTED'}
              </span>
            </div>
          </div>
        </div>

        <!-- ACTIONS -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>ACTIONS</h3>
          <button
            class={css({
              borderWidth: '1px',
              borderColor: 'amber.500',
              paddingX: '12px',
              paddingY: '8px',
              marginY: '8px',
              fontSize: '12px',
              color: 'amber.500',
              backgroundColor: 'transparent',
              width: 'full',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '8px',
              _hover: {
                backgroundColor: 'amber.500',
                color: 'gray.900',
              },
            })}
            onclick={() => (impersonateModalOpen = true)}
            type="button"
          >
            IMPERSONATE USER
          </button>

          <button
            class={css({
              borderWidth: '1px',
              borderColor: 'amber.500',
              paddingX: '12px',
              paddingY: '8px',
              marginY: '8px',
              fontSize: '12px',
              color: 'amber.500',
              backgroundColor: 'transparent',
              width: 'full',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '8px',
              _hover: {
                backgroundColor: 'amber.500',
                color: 'gray.900',
              },
            })}
            onclick={async () => {
              const amount = Number.parseInt(prompt('Enter the amount of credit to give: ') || '');

              if (!Number.isNaN(amount)) {
                await adminGiveCredit({ userId: $query.adminUser.id, amount });
                query.load({ userId: $query.adminUser.id });
                alert(`${amount} points given to user ${$query.adminUser.name}`);
              }
            }}
            type="button"
          >
            GIVE CREDIT
          </button>
        </div>
      </div>
    </div>

    <AdminModal
      actions={{
        cancel: {},
        confirm: {
          label: 'CONFIRM IMPERSONATE',
          onclick: handleImpersonate,
          variant: 'primary',
        },
      }}
      title="CONFIRM IMPERSONATION"
      bind:open={impersonateModalOpen}
    >
      <div class={css({ marginBottom: '16px' })}>
        <p class={css({ marginBottom: '8px' })}>ARE YOU SURE YOU WANT TO IMPERSONATE THIS USER?</p>
        <p class={css({ color: 'amber.400' })}>
          USER: {$query.adminUser.name.toUpperCase()} ({$query.adminUser.email})
        </p>
      </div>
    </AdminModal>
  {/if}
</div>
