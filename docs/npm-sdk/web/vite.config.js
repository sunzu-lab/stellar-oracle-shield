// Use polling instead of inotify file watching, so the dev server doesn't hit
// the OS inotify-instance limit (EMFILE) — no sudo / sysctl needed.
// Only source files are watched; node_modules / dist / .git are ignored, so
// polling stays cheap.
export default {
  server: {
    watch: {
      usePolling: true,
      interval: 300,
      ignored: ["**/node_modules/**", "**/dist/**", "**/.git/**"],
    },
  },
};
