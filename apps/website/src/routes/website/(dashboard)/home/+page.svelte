<script lang="ts">
  import FilePenIcon from '~icons/lucide/file-pen';
  import { graphql } from '$graphql';
  import { Helmet, Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const query = graphql(`
    query HomePage_Query {
      me @required {
        id

        sites {
          id

          firstEntity(type: POST) {
            id
            slug
          }
        }
      }
    }
  `);
</script>

<Helmet title="홈" />

<div class={center({ flexDirection: 'column', size: 'full' })}>
  <Icon style={css.raw({ size: '60px' })} icon={FilePenIcon} />
  <div class={css({ textAlign: 'center' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>첫 페이지를 만들어보세요</h1>
    <p class={css({ fontSize: '15px', color: 'gray.500' })}>아직 페이지가 없어요.</p>
  </div>
</div>
