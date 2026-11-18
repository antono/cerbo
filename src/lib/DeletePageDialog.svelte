<script lang="ts">
  /**
   * Delete-page confirmation. Mounted by the root layout, not by the tree, so
   * "Delete page" from the toolbar page menu or the Del hotkey shows it even
   * when no tree is mounted (narrow layout, hidden sidebar).
   */
  import { app, deletePage } from './stores.svelte';
  import ConfirmationDialog from './ConfirmationDialog.svelte';

  let deleting = $state(false);

  const page = $derived(app.pages.find((p) => p.uuid === app.confirmDeleteUuid));

  async function handleDelete() {
    if (!app.confirmDeleteUuid) return;
    deleting = true;
    try {
      await deletePage(app.confirmDeleteUuid);
      app.confirmDeleteUuid = null;
    } catch (_) {
      // error set in store
    } finally {
      deleting = false;
    }
  }
</script>

<ConfirmationDialog
  title="Delete page?"
  message={`This will permanently delete "${page?.title ?? app.confirmDeleteUuid}" and all its assets.`}
  confirmLabel={deleting ? 'Deleting…' : 'Delete'}
  confirmDisabled={deleting}
  onClose={() => { app.confirmDeleteUuid = null; }}
  onConfirm={handleDelete}
/>
