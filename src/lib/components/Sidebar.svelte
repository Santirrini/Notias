<script lang="ts">
  // ponytail: routes wired in Phase 1; placeholder items shown only.
  const items = [
    { href: "/", label: "Today" },
    { href: "/notes", label: "Notes" },
    { href: "/chat", label: "Chat" },
    { href: "/settings", label: "Settings" },
  ];
</script>

<nav>
  <ul>
    {#each items as it}
      <li><a href={it.href}>{it.label}</a></li>
    {/each}
  </ul>
</nav>

<style>
  nav { width: 200px; border-right: 1px solid #eee; padding: 1rem; }
  ul { list-style: none; padding: 0; margin: 0; }
  li { padding: 0.25rem 0; }
  a { color: inherit; text-decoration: none; }
  a:hover { text-decoration: underline; }
</style>
