<script lang="ts">
  import { quitApp } from './stores.svelte';
  import { QUIT_PROMPT } from './window-chrome';
  import ConfirmationDialog from './ConfirmationDialog.svelte';

  let { onClose }: { onClose: () => void } = $props();
</script>

<ConfirmationDialog
  title={QUIT_PROMPT.title}
  message={QUIT_PROMPT.message}
  confirmLabel={QUIT_PROMPT.confirmLabel}
  {onClose}
  onConfirm={quitApp}
/>
