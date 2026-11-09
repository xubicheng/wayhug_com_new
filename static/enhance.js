// 可选增强：防重复提交。没有它，页面依然完整可用。
(function () {
  document.addEventListener('submit', function (event) {
    var form = event.target;
    if (!(form instanceof HTMLFormElement) || form.dataset.submitting === '1') return;
    form.dataset.submitting = '1';
    var btn = form.querySelector('button[type="submit"]');
    if (btn) {
      btn.setAttribute('aria-busy', 'true');
      btn.style.opacity = '0.6';
      btn.style.pointerEvents = 'none';
    }
    window.setTimeout(function () {
      form.dataset.submitting = '';
      if (btn) {
        btn.removeAttribute('aria-busy');
        btn.style.opacity = '';
        btn.style.pointerEvents = '';
      }
    }, 8000);
  });
})();
