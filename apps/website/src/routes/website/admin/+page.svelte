<script lang="ts">
  import { graphql } from '$graphql';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  const query = graphql(`
    query AdminHome_Query {
      me @required {
        id
        name
      }
    }
  `);
</script>

<div class={center({ height: 'full', backgroundColor: 'gray.900' })}>
  <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '24px' })}>
    <div
      class={css({
        borderWidth: '2px',
        borderStyle: 'solid',
        borderColor: 'amber.500',
        padding: '24px',
      })}
    >
      <h1 class={css({ fontSize: '18px', color: 'amber.500', fontWeight: 'normal', textAlign: 'center' })}>TYPIE ADMIN SYSTEM</h1>
    </div>
    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '12px' })}>
      <p class={css({ fontFamily: 'mono', fontSize: '13px', color: 'amber.500' })}>
        WELCOME, {$query.me.name.toUpperCase()}
      </p>
      <p class={css({ fontFamily: 'mono', fontSize: '11px', color: 'amber.500' })}>SYSTEM READY_</p>
    </div>
  </div>
</div>
