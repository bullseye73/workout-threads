<script lang="ts">
  // ask()가 띄우는 확인창. App에 한 번만 둔다.
  import * as AlertDialog from '$lib/components/ui/alert-dialog'
  import { buttonVariants } from '$lib/components/ui/button'
  import { answer, confirmState } from '$lib/confirm.svelte'
</script>

<AlertDialog.Root open={confirmState.open} onOpenChange={o => { if (!o) answer(false) }}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>{confirmState.title}</AlertDialog.Title>
      {#if confirmState.description}<AlertDialog.Description>{confirmState.description}</AlertDialog.Description>{/if}
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel onclick={() => answer(false)}>{confirmState.cancel ?? '취소'}</AlertDialog.Cancel>
      <AlertDialog.Action class={confirmState.destructive ? buttonVariants({ variant: 'destructive' }) : ''} onclick={() => answer(true)}>{confirmState.action}</AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
