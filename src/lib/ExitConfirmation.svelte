<script lang="ts">
  import { quitApp } from './stores.svelte';
  import ConfirmationDialog from './ConfirmationDialog.svelte';

  let { onClose }: { onClose: () => void } = $props();
</script>

<ConfirmationDialog
  title="Quit Application?"
  message="Are you sure you want to exit Cerbo?"
  confirmLabel="Quit"
  {onClose}
  onConfirm={quitApp}
/>
