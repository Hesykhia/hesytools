<script lang="ts">
  // Renders a stored template (base64 RGBA) as a small image.
  import type { Template } from "$lib/api";

  let { template, height = 26 }: { template: Template; height?: number } = $props();

  let src = $derived.by(() => {
    try {
      const bytes = Uint8ClampedArray.from(atob(template.data), (c) => c.charCodeAt(0));
      const canvas = document.createElement("canvas");
      canvas.width = template.w;
      canvas.height = template.h;
      canvas.getContext("2d")!.putImageData(new ImageData(bytes, template.w, template.h), 0, 0);
      return canvas.toDataURL();
    } catch {
      return "";
    }
  });
</script>

{#if src}
  <img {src} alt="What to look for" style:height="{height}px" title="{template.w}×{template.h}" />
{/if}

<style>
  img {
    display: inline-block;
    border-radius: 5px;
    image-rendering: pixelated;
    box-shadow:
      0 0 0 1px rgba(255, 255, 255, 0.25),
      0 2px 8px rgba(0, 0, 0, 0.5);
    vertical-align: middle;
  }
</style>
