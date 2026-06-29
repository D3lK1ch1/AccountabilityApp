chrome.runtime.onMessage.addListener((message) => {
  if (message.type === 'accountability_unblock') {
    document.getElementById('accountability-block-overlay')?.remove();
    return;
  }
  if (message.type !== 'accountability_block') return;

  document.getElementById('accountability-block-overlay')?.remove();
  const overlay = document.createElement('div');
  overlay.id = 'accountability-block-overlay';
  overlay.style.position = 'fixed';
  overlay.style.inset = '0';
  overlay.style.zIndex = '2147483647';
  overlay.style.background = 'rgba(12, 14, 20, 0.96)';
  overlay.style.color = '#fff';
  overlay.style.display = 'grid';
  overlay.style.placeItems = 'center';
  overlay.style.fontFamily = 'system-ui, sans-serif';
  overlay.innerHTML = `
    <section style="max-width:520px;padding:28px;text-align:center">
      <h1 style="font-size:28px;margin:0 0 12px">Category limit reached</h1>
      <p style="font-size:16px;line-height:1.5;margin:0 0 18px">${message.category_name || 'This category'} is blocked until you close or leave this tab.</p>
      <p style="font-size:13px;color:#b6c2d2;margin:0">Used ${formatSeconds(message.used_seconds)} of ${formatSeconds(message.limit_seconds)}.</p>
    </section>
  `;
  document.documentElement.appendChild(overlay);
});

function formatSeconds(seconds) {
  const minutes = Math.floor((seconds || 0) / 60);
  const hours = Math.floor(minutes / 60);
  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  return `${minutes}m`;
}
